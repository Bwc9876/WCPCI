import "katex/dist/katex.css";
import katex from "katex";

export default () => {
    document.querySelectorAll("#rendered-md code.math-inline").forEach((block) => {
        katex.render(block.textContent ?? "", block as HTMLElement, { throwOnError: false });
    });
    document.querySelectorAll("#rendered-md pre code.math-display").forEach((block) => {
        katex.render(block.textContent ?? "", block as HTMLElement, {
            throwOnError: false,
            displayMode: true
        });
    });
};
