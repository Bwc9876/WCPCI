export const variable = (expression: string, debugEval?: string) =>
    import.meta.env.DEV ? debugEval ?? expression : `{{ ${expression} }}`;
export const tag = (expression: string, whitespace?: boolean) =>
    `{%${whitespace ? "" : "-"} ${expression} ${whitespace ? "" : "-"}%}`;
export const teraIf = (condition: string, t: string, f?: string) =>
    `${tag(`if ${condition}`)}${t}${f ? `${tag("else")}${f}` : ""}`;
