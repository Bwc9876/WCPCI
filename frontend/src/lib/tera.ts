export const variable = (expression: string) => `{{ ${expression} }}`;
export const tag = (expression: string) => `{%- ${expression} -%}`;
export const teraIf = (condition: string, t: string, f?: string) =>
    `${tag(`if ${condition}`)}${t}${f ? `${tag("else")}${f}` : ""}`;
