import "highlight.js/styles/default.css";
import "@/styles/highlight-theme.scss";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import plaintext from "highlight.js/lib/languages/plaintext";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";
import java from "highlight.js/lib/languages/java";
import kotlin from "highlight.js/lib/languages/kotlin";
import haskell from "highlight.js/lib/languages/haskell";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("py", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("rs", rust);
hljs.registerLanguage("java", java);
hljs.registerLanguage("kotlin", kotlin);
hljs.registerLanguage("haskell", haskell);
hljs.registerLanguage("c", c);
hljs.registerLanguage("cpp", cpp);
hljs.registerLanguage("cc", cpp);
hljs.registerLanguage("cxx", cpp);
hljs.registerLanguage("example", plaintext);

const makePreCode = (text: string): HTMLPreElement => {
    const pre = document.createElement("pre");
    const code = document.createElement("code");
    code.classList.add("language-example");
    code.textContent = text;
    pre.appendChild(code);
    return pre;
};

export default (
    selectorPrefix?: string,
    onRunExample?: (input: string) => void,
    exampleButtonTemplate?: HTMLButtonElement
) => {
    document
        .querySelectorAll(
            `${selectorPrefix !== undefined ? selectorPrefix + " " : ""}pre code:not(.language-math):not(language-example)`
        )
        .forEach((block) => {
            hljs.highlightElement(block as HTMLElement);
        });

    if (exampleButtonTemplate) {
        document
            .querySelectorAll(
                `${selectorPrefix !== undefined ? selectorPrefix + " " : ""}pre code.language-example`
            )
            .forEach((block) => {
                const wrapperElem = document.createElement("div");
                wrapperElem.classList.add("relative");
                const clonedButton = exampleButtonTemplate.cloneNode(true) as HTMLButtonElement;
                clonedButton.removeAttribute("id");
                clonedButton.classList.remove("hidden");
                clonedButton.onclick = () => {
                    onRunExample?.(block.textContent ?? "");
                };
                wrapperElem.appendChild(clonedButton);
                const newBlock = makePreCode(block.textContent ?? "");
                wrapperElem.appendChild(newBlock);
                block.parentElement!.replaceWith(wrapperElem);
                hljs.highlightElement(newBlock.childNodes[0] as HTMLElement);
            });
    }
};
