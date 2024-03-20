import * as monaco from "monaco-editor";
import monacoDarkTheme from "@/lib/wcpc-monaco-dark.json";
import monacoLightTheme from "@/lib/wcpc-monaco-light.json";

monaco.editor.defineTheme("wcpc-dark", monacoDarkTheme as any);
monaco.editor.defineTheme("wcpc-light", monacoLightTheme as any);

declare global {
    interface Window {
        stylesClone?: HTMLStyleElement;
    }
}

export type CodeInfo = {
    [lang: string]: {
        name: string;
        tablerIcon: string;
        monacoContribution: string;
        defaultCode: string;
        fileName: string;
    };
};

export const makeIconUrl = (name: string) =>
    `https://raw.githubusercontent.com/tabler/tabler-icons/main/icons/outline/${name}.svg`;

export default (
    codeInfo: CodeInfo,
    defaultLanguage: string,
    problemId: string,
    languageDropdown: HTMLSelectElement,
    colorScheme: string,
    editorElem: HTMLElement,
    languageIcon: HTMLImageElement
) => {
    let editor: monaco.editor.IStandaloneCodeEditor | null = null;
    let currentLanguage = defaultLanguage;

    languageDropdown.onchange = (e) => {
        const lang = (e.target as HTMLSelectElement).value;
        const langInfo = codeInfo[lang];
        if (langInfo) {
            currentLanguage = lang;
            languageIcon.src = makeIconUrl(langInfo.tablerIcon);
            if (editor) {
                const storedCode = JSON.parse(
                    window.localStorage.getItem(`problem-${problemId}-${lang}-code`) ?? "null"
                );
                editor.setValue(storedCode ?? langInfo.defaultCode);
                monaco.editor.setModelLanguage(editor.getModel()!, langInfo.monacoContribution);
                window.localStorage.setItem(
                    `problem-${problemId}-code`,
                    JSON.stringify([storedCode, lang])
                );
            }
        }
    };

    const [storedCode, storedLang] = JSON.parse(
        window.localStorage.getItem(`problem-${problemId}-code`) ?? "[null, null]"
    );

    const lang = Object.keys(codeInfo).includes(storedLang ?? "") ? storedLang : defaultLanguage;

    const langInfo = codeInfo[lang];

    languageDropdown.value = lang;
    languageIcon.src = makeIconUrl(langInfo.tablerIcon);
    setTimeout(() => languageIcon.classList.remove("opacity-0"), 300);

    const mql = matchMedia("(prefers-color-scheme: dark)");

    const themeVariant = colorScheme === "system" ? (mql.matches ? "dark" : "light") : colorScheme;

    if (colorScheme === "system") {
        mql.addEventListener("change", (mql) => {
            if (editor) {
                monaco.editor.setTheme(mql.matches ? "wcpc-dark" : "wcpc-light");
            }
        });
    }

    editor = monaco.editor.create(editorElem as HTMLElement, {
        value: storedCode ?? langInfo.defaultCode,
        theme: `wcpc-${themeVariant}`,
        language: langInfo.monacoContribution,
        automaticLayout: true,
        minimap: { enabled: false }
    });

    if (window.stylesClone) {
        const newStyles = window.stylesClone.cloneNode(true) as HTMLStyleElement;
        document.head.appendChild(newStyles);
    } else {
        window.stylesClone = document.head.querySelector(".monaco-colors") as HTMLStyleElement;
    }

    let currentTimeout: number | undefined = undefined;
    let oldLang = lang;
    editor!.onDidChangeModelContent(() => {
        if (currentTimeout) {
            clearTimeout(currentTimeout);
        }
        currentTimeout = setTimeout(() => {
            if (editor && oldLang === currentLanguage) {
                const text = editor.getValue();
                const language = editor.getModel()?.getLanguageId();
                window.localStorage.setItem(
                    `problem-${problemId}-code`,
                    JSON.stringify([text, language])
                );
                window.localStorage.setItem(
                    `problem-${problemId}-${currentLanguage}-code`,
                    JSON.stringify(text)
                );
            }
        }, 1000) as unknown as number;
        oldLang = currentLanguage!;
    });
    console.debug("Instantiated Monaco editor");

    return [editor, () => currentLanguage];
};
