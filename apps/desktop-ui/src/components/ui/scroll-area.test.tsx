import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

import { ScrollArea } from "./scroll-area";

describe("ScrollArea", () => {
  test("renders the viewport content wrapper as a block so it tracks the viewport width", () => {
    const markup = renderToStaticMarkup(
      <ScrollArea>
        <p>row</p>
      </ScrollArea>,
    );

    // Radix puts an inline `display: table` on the viewport's first child,
    // which lets wide rows grow past the panel instead of truncating.
    expect(markup).toContain("display:table");
    expect(markup).toContain("data-radix-scroll-area-viewport");
    expect(markup).toContain("[&amp;&gt;div]:block!");
  });
});
