declare module "js-beautify" {
  export interface JSBeautifyOptions {
    indent_size?: number;
    preserve_newlines?: boolean;
    max_preserve_newlines?: number;
    wrap_line_length?: number;
    end_with_newline?: boolean;
  }

  export function js(source: string, options?: JSBeautifyOptions): string;
}
