import "highlight.js/styles/default.css";
import "highlight.js/styles/an-old-hope.css";
import hljs from "highlight.js/lib/core";
import javascript from "highlight.js/lib/languages/javascript";
import plaintext from "highlight.js/lib/languages/plaintext";
import python from "highlight.js/lib/languages/python";
import rust from "highlight.js/lib/languages/rust";

hljs.registerLanguage("javascript", javascript);
hljs.registerLanguage("js", javascript);
hljs.registerLanguage("python", python);
hljs.registerLanguage("py", python);
hljs.registerLanguage("rust", rust);
hljs.registerLanguage("rs", rust);
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
    onRunExample: (input: string) => void,
    exampleButtonTemplate?: HTMLButtonElement
) => {
    document
        .querySelectorAll("#rendered-md pre code:not(.language-math):not(language-example)")
        .forEach((block) => {
            hljs.highlightElement(block as HTMLElement);
        });

    if (exampleButtonTemplate) {
        document.querySelectorAll("#rendered-md pre code.language-example").forEach((block) => {
            const wrapperElem = document.createElement("div");
            wrapperElem.classList.add("relative");
            const clonedButton = exampleButtonTemplate.cloneNode(true) as HTMLButtonElement;
            clonedButton.removeAttribute("id");
            clonedButton.classList.remove("hidden");
            clonedButton.onclick = () => {
                onRunExample(block.textContent ?? "");
            };
            wrapperElem.appendChild(clonedButton);
            const newBlock = makePreCode(block.textContent ?? "");
            wrapperElem.appendChild(newBlock);
            block.parentElement!.replaceWith(wrapperElem);
            hljs.highlightElement(newBlock.childNodes[0] as HTMLElement);
        });
    }
};
