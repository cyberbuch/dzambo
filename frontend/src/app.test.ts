import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { createElement } from "react";
import { describe, expect, it } from "vitest";

import App from "@/App";
import * as session from "@/stores/session";

describe("App hydrate", () => {
  it("след hydrate без токен показва логин екрана", async () => {
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      createElement(QueryClientProvider, { client: qc }, createElement(App)),
    );
    await waitFor(() =>
      expect(screen.getByText("dzambo — вход")).toBeTruthy(),
    );
    expect(session.useSession.getState().hydrated).toBe(true);
  });
});
