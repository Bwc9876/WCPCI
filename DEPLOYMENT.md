# Deployment Guide

This guide is for deploying an instance of WCPC on a server. This serve __must__ be using a Linux distribution. This guide is written in a way that should be distro-agnostic but certain steps may require slight modifications depending on the distribution.

Table of contents:

- [Deployment Guide](#deployment-guide)
  - [Prerequisites](#prerequisites)
  - [Distribution](#distribution)
  - [Configuration](#configuration)
    - [Profile](#profile)
    - [Base](#base)
    - [TLS](#tls)
    - [Database Configuration](#database-configuration)
    - [OAuth Configuration](#oauth-configuration)
    - [SAML Configuration](#saml-configuration)
    - [Run](#run)
      - [Languages](#languages)
  - [Database](#database)
  - [OAuth](#oauth)
  - [SAML](#saml)

## Prerequisites

The server you're running on will need a few dependencies:

- libxml2
- libxslt
- xmlsec (for SAML assertion signing / verification)
- libiconv
- openssl
- libsqlite3

## Distribution

To distribute this as a package you'll need to ensure that the above dependencies are installed on the target machine.
In addition the following folder need to go along with the package:

- `migrations/`
- `frontend/dist/`
- `public/`

The exact paths to these folders can be changed in the config (covered in a bit) but just make sure they're packaged in some way with the binary.

## Configuration

This section will cover all aspects of configuration, some of which are required and some of which are optional.

Configuration is done via the `Rocket.toml` file that needs to either.

A. Be in the same directory as the binary
B. Be in a parent directory of the binary
C. Be directly specified in the `ROCKET_CONFIG` environment variable

In addition, any __top-level__ (described in the "Base" section) settings can be specified as an environment variable. Just prepend `ROCKET_` and put the name in all caps (e.g `url` becomes `ROCKET_URL`).

### Profile

Rocket uses profiles to make configuration different between what stage of development you're in. Most of the time (and by default), Rocket is always in production mode. We'd recommend setting the `ROCKET_ENV` environment variable to `development` when developing and `prod` when deploying, or `stage` if you're deploying to a staging environment.

### Base

- `cli_colors` - Whether to use colors and emoji in the CLI. (by default this is `true`)
- `ident` - The identifier to send in the `Server` header, `WCPC` by default
- `ip_header` - IP header to use for getting the user's IP address. By default this is `X-Real-IP` but can be changed to `X-Forwarded-For` if you're behind a reverse proxy.
- `address` - The address to bind the application to.
- `port` - The port to bind the application to.
- `workers` - The number of workers to spawn for the application. (by default this is CPU count * 2)
- `url` - The URL that the application will be running on. This option should __not__ have a trailing slash.
- `secret_key` - The secret key used for signing cookies. Prefer to set this as an environment variable (`ROCKET_SECRET_KEY`)
- `template_dir` - The directory to use to grab templates generated from the `frontend` folder, this is by default `frontend/dist/` but can (and most likely will have to) be changed.
- `public_dir` - The directory to use to grab static files, this is by default `public/` but can (and most likely will have to) be changed.
- `admins` - A list of __email addresses__ to be considered as admins. These users will have access to the admin panel and various other features.
- `timezone` - The timezone to use for the application. The application will by default try to use the user's but if that fails it will fall back to this.

### TLS

For TLS configuration use the `tls` object. This object has the following fields:

- `tls.certs` - The path to the certificate chain file
- `tls.key` - The path to the private key file
- `tls.ciphers` - An array of ciphers to use for the TLS connection. (by default this is all TLS v1.3 and TLS v1.2 suites)
- `tls.prefer_server_cipher_order` - Whether to prefer the server's cipher order over the client's cipher order. (by default this is `false`)

### Database Configuration

- `databases.sqlite_db.url` - The file path to the SQLite database. See the [Database section](#database) for more information.

### OAuth Configuration

- `oauth.github` - This is the OAuth configuration for GitHub. See the [OAuth section](#oauth) for more information.
  - `provider` - Set this to `GitHub`
  - `redirect_uri` - Set to your URL and then `/auth/github/callback`
  - `client_id` - The client ID for the GitHub OAuth application.
  - `client_secret` - The client secret for the GitHub OAuth application.
- `oauth.google` - This is the OAuth configuration for Google. See the [OAuth section](#oauth) for more information.
  - `provider` - Set this to `Google`
  - `redirect_uri` - Set to your URL and then `/auth/google/callback`
  - `client_id` - The client ID for the Google OAuth application.
  - `client_secret` - The client secret for the Google OAuth application.

### SAML Configuration

See the [SAML section](#saml) for more information.

- `saml.entity_id` - The entity ID for the SAML service provider, this can be anything but is recommended to be a URI by the SAML spec, this will need to be shared with the identity provider.
- `saml.idp_metadata_url` - The URL the application will use to fetch the identity provider's metadata, this should be an XML endpoint.
- `saml.cert` - The certificate to use to sign assertions with. This should be a PEM encoded certificate.
- `saml.key` - The private key to use to sign assertions with. This should be a PEM encoded private key.
- `saml.contact_name` - The name of the contact person for the SAML service provider.
- `saml.contact_email` - The email of the contact person for the SAML service provider.
- `saml.contact_telephone` - The telephone number of the contact person for the SAML service provider.
- `saml.organization` - The organization name for the SAML service provider.
- `saml.attrs` - Defines how to map attribute assertions to user data. By default these are the OpenID Connect attributes but can be changed to match the identity provider's attributes.
  - `display_name` - The attribute name to use for the display name.
  - `email` - The attribute name to use for the email.

### Run

- `max_program_length` - The max length of a program in bytes. This is to prevent massive programs from being saved and causing issues.
- `default_language` - A key from the `languages` object (described below) to use as the default language for new programs.

#### Languages

`run.languages` is a map of language keys to language objects. These objects contain the following fields:

- `name` - The name of the language for display
- `default_code` - The default code to use for the language (hint: you can use `"""` in toml to make multiline strings)
- `tabler_icon` The icon (from [tabler icons](https://tabler.io/icons)) to use for this language, not this must be the full name of the icon (e.g. `brand-python`)
- `monaco_contribution` - The monaco editor contribution to use for this language, this is the language ID for monaco (e.g. `python`)
- `file_name` - The name of the file to save the user's code to when running a submission.
- `compile_cmd` - The command to use to compile the code, this can be left blank if the language doesn't need to be compiled, but we'd recommend setting it to do static analysis of an interpreted language to be fair to all users. The source file is named as whatever is in `file_name`.
- `run_cmd` - The command to use to run the code. This command will be passed the input of the testcase as stdin and should output the result of the program to stdout. The source file is named as whatever is in `file_name`.

## Database

The database must be a SQLite database, the application will create the file if it isn't present.

On any updates to the application it should be able to migrate the database automatically, __but it may still be a good idea to back it up before updating__.

## OAuth

For Oauth, see the [section in configuration](#oauth-configuration) for what keys and values to use.

You'll need to create GitHub and Google OAuth apps in order to use them as providers.

See the [GitHub OAuth docs](https://docs.github.com/en/developers/apps/building-oauth-apps) and the [Google OAuth docs](https://developers.google.com/identity/protocols/oauth2) for more information.

## SAML

For SAML you'll need to configure your identity provider to trust the application. The application will need to know the entity ID and the metadata URL of the identity provider, as well as a certificate and private key to sign assertions with.

This application supports HTTP-Post bindings, and at the moment only support SP-initiated SSO. More bindings and features may be added in the future.
