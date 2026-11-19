import { describe, expect, it } from "vitest";

import { formatApiDetail } from "@/lib/api-detail";

describe("api detail форматиране (договорът от ytbuch)", () => {
  it("извлича низова грешка", () => {
    expect(formatApiDetail({ detail: "грешка" }, "fallback")).toBe("грешка");
  });

  it("извлича списък от грешки на полета", () => {
    const data = { detail: [{ loc: ["body", "name"], msg: "липсва" }] };
    expect(formatApiDetail(data, "fallback")).toBe("body.name: липсва");
  });

  it("връща fallback при празен отговор", () => {
    expect(formatApiDetail(null, "HTTP 500")).toBe("HTTP 500");
  });
});
