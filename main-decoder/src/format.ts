import { js as beautify } from "js-beautify";

export function formatJavaScript(source: string): string {
  const formatted = beautify(source, {
    indent_size: 2,
    preserve_newlines: true,
    max_preserve_newlines: 2,
    wrap_line_length: 0,
    end_with_newline: true,
  });

  return formatted.endsWith("\n") ? formatted : `${formatted}\n`;
}
