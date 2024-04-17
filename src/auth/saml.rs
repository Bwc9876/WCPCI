use std::collections::HashMap;

use log::warn;
use rocket::{
    fairing::AdHoc, form::Form, get, http::CookieJar, http::Status, post, response::Redirect,
    routes, FromForm, State,
};
use samael::{
    metadata::{ContactPerson, EntityDescriptor},
    service_provider::{ServiceProvider, ServiceProviderBuilder},
};
use serde::Deserialize;

use crate::{db::DbConnection, run::CodeInfo};

use super::users::User;

fn cn_oid() -> String {
    "urn:oid:2.5.4.3".to_string()
}

fn email_oid() -> String {
    "urn:oid:0.9.2342.19200300.100.1.3".to_string()
}

#[derive(Debug, Deserialize)]
struct AttrOptions {
    #[serde(default = "cn_oid")]
    display_name: String,
    #[serde(default = "email_oid")]
    email: String,
}

#[derive(Debug, Deserialize)]
struct SamlOptions {
    entity_id: String,
    idp_meta_url: Option<String>,
    certificate: Option<String>,
    private_key: Option<String>,
    contact_person: Option<String>,
    contact_email: Option<String>,
    contact_telephone: Option<String>,
    organization_name: Option<String>,
    attrs: AttrOptions,
}

const PREFERRED_SSO_BINDING: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect";
const NAME_ID_FORMAT: &str = "urn:oasis:names:tc:SAML:2.0:nameid-format:persistent";

impl SamlOptions {
    pub async fn create_service_provider(
        &self,
        url_prefix: &str,
    ) -> Result<ServiceProvider, String> {
        let mut sp = ServiceProviderBuilder::default();
        sp.entity_id(self.entity_id.clone())
            .allow_idp_initiated(true)
            .acs_url(format!("{}/auth/saml/acs", url_prefix))
            .slo_url(format!("{}/auth/saml/slo", url_prefix))
            .metadata_url(format!("{}/auth/saml/metadata", url_prefix))
            .authn_name_id_format(Some(NAME_ID_FORMAT.to_string()))
            .contact_person(ContactPerson {
                sur_name: self.contact_person.clone(),
                email_addresses: self.contact_email.as_ref().map(|e| vec![e.clone()]),
                telephone_numbers: self.contact_telephone.as_ref().map(|t| vec![t.clone()]),
                company: self.organization_name.clone(),
                ..Default::default()
            });

        if let Some(idp_meta_url) = &self.idp_meta_url {
            let resp = reqwest::get(idp_meta_url)
                .await
                .map_err(|e| format!("Couldn't fetch IDP metadata: {e}"))?;
            let text = resp
                .text()
                .await
                .map_err(|e| format!("Couldn't read IDP metadata: {e}"))?;
            let idp_meta: EntityDescriptor = samael::metadata::de::from_str(&text)
                .map_err(|e| format!("Couldn't parse IDP metadata: {e}"))?;
            sp.idp_metadata(idp_meta);
        }

        if let Some(cert_path) = &self.certificate {
            let cert_raw = std::fs::read_to_string(cert_path)
                .map_err(|e| format!("Couldn't read certificate: {e}"))?;
            let cert = openssl::x509::X509::from_pem(cert_raw.as_bytes())
                .map_err(|e| format!("Couldn't parse certificate: {e}"))?;
            sp.certificate(cert);
        }

        if let Some(private_key_path) = &self.private_key {
            let private_key = std::fs::read_to_string(private_key_path)
                .map_err(|e| format!("Couldn't read private key: {e}"))?;
            let key = openssl::rsa::Rsa::private_key_from_pem(private_key.as_bytes())
                .map_err(|e| format!("Couldn't parse private key: {e}"))?;
            sp.key(key);
        }

        let sp = sp.build().map_err(|e| e.to_string())?;

        if sp.sso_binding_location(PREFERRED_SSO_BINDING).is_none() {
            Err(format!(
                "IDP doesn't support the preferred SSO binding: {PREFERRED_SSO_BINDING}"
            ))
        } else {
            Ok(sp)
        }
    }
}

#[get("/login")]
async fn login(
    sp: &State<ServiceProvider>,
    relay_url: &State<UrlPrefixGuard>,
) -> Result<Redirect, String> {
    let base = sp.sso_binding_location(PREFERRED_SSO_BINDING).unwrap();
    let req = sp
        .make_authentication_request(&base)
        .map_err(|e| format!("Couldn't Create Authn Request: {e}"))?;
    let relay = relay_url.0.clone();
    let url = if let Some(key) = sp.key.as_ref() {
        let key_der = key
            .private_key_to_der()
            .map_err(|e| format!("Couldn't convert private key to DER: {e}"))?;
        req.signed_redirect(&relay, &key_der)
    } else {
        req.redirect(&relay)
    }
    .map_err(|e| format!("Couldn't Generate Redirect: {e}"))?
    .ok_or_else(|| "Couldn't Generate Redirect".to_string())?;

    Ok(Redirect::to(url.to_string()))
}

#[get("/metadata")]
async fn metadata(sp: &State<ServiceProvider>) -> Result<String, String> {
    sp.metadata()
        .map_err(|e| format!("Couldn't generate metadata: {e}"))
        .and_then(|x| {
            x.to_xml()
                .map_err(|e| format!("Couldn't serialize metadata: {e}"))
        })
}

#[derive(FromForm, Debug)]
struct SamlAcsForm {
    #[field(name = "SAMLResponse")]
    saml_response: String,
}

#[post("/acs", data = "<form>")]
async fn acs(
    mut db: DbConnection,
    sp: &State<ServiceProvider>,
    so: &State<SamlOptions>,
    form: Form<SamlAcsForm>,
    code_info: &State<CodeInfo>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, Status> {
    let raw = form.into_inner().saml_response;

    let assertion = sp.parse_base64_response(&raw, None).map_err(|e| {
        warn!("Couldn't parse or validate SAML response: {e}");
        Status::BadRequest
    })?;

    if let Some(attrs) = assertion.attribute_statements {
        let attrs_map = attrs
            .into_iter()
            .flat_map(|x| {
                x.attributes
                    .into_iter()
                    .filter_map(|a| {
                        a.name.and_then(|name| {
                            a.values
                                .into_iter()
                                .next()
                                .and_then(|v| v.value)
                                .map(|value| (name, value))
                        })
                    })
                    .collect::<HashMap<_, _>>()
            })
            .collect::<HashMap<_, _>>();

        let id = assertion.subject.and_then(|s| s.name_id.map(|n| n.value));

        if let (Some(id), Some(display_name), Some(email)) = (
            id,
            attrs_map.get(&so.attrs.display_name),
            attrs_map.get(&so.attrs.email),
        ) {
            let user = User::temporary(
                id,
                email.clone(),
                display_name.clone(),
                &code_info.run_config.default_language,
            );
            match user.login_or_register(&mut db, cookies).await {
                Err(why) => {
                    warn!("Couldn't log / register in user: {why}");
                    Err(Status::InternalServerError)
                }
                Ok((_user, is_new)) => {
                    Ok(Redirect::to(if is_new { "/settings/profile" } else { "/" }))
                }
            }
        } else {
            warn!(
                "No display name or email found in SAML response, looked for {} and {}",
                so.attrs.display_name, so.attrs.email
            );
            Err(Status::BadRequest)
        }
    } else {
        warn!("No attributes found in SAML response");
        Err(Status::BadRequest)
    }
}

struct UrlPrefixGuard(String);

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("SAML Auth", |rocket| async {
        let figment = rocket.figment();
        let url = figment
            .extract_inner::<String>("url")
            .expect("Couldn't extract URL");
        let saml_options = figment.extract_inner::<SamlOptions>("saml").ok();
        if let Some(saml_options) = saml_options {
            let sp = saml_options
                .create_service_provider(&url)
                .await
                .expect("Couldn't create service provider");
            rocket
                .manage(sp)
                .manage(saml_options)
                .manage(UrlPrefixGuard(url))
                .mount("/auth/saml", routes![login, metadata, acs])
        } else {
            warn!("No / Invalid SAML options found, users won't be able to authenticate with SAML");
            rocket
        }
    })
}
