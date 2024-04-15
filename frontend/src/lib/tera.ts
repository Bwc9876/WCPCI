export const variable = (expression: string, debugEval?: string) =>
    import.meta.env.DEV ? debugEval ?? expression : `{{ ${expression} }}`;
export const tag = (expression: string, whitespace?: boolean) =>
    `{%${whitespace ? "" : "-"} ${expression} ${whitespace ? "" : "-"}%}`;
export const teraIf = (condition: string, t: string, f?: string) =>
    `${tag(`if ${condition}`)}${t}${f ? `${tag("else")}${f}` : ""}`;

export const themeClass = (light: string, dark: string, system?: string) => {
    return import.meta.env.DEV
        ? system ?? dark
        : `${tag("if scheme == 'Dark'", true)}${dark}${tag("elif scheme == 'Light'")}${light}${tag("else")}${system ?? dark}${tag("endif", true)}`;
};
