import { useState, type FormEvent } from "react";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, ApiError } from "@/lib/api";
import type { CompanyDto } from "@/bindings/CompanyDto";
import type { CompanySettingsDto } from "@/bindings/CompanySettingsDto";

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm text-muted-foreground">{title}</CardTitle>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export default function SettingsPage() {
  const qc = useQueryClient();
  const company = useQuery({
    queryKey: ["active-company"],
    queryFn: async () => {
      const res = await api.get<CompanyDto | Record<string, never>>("/v1/active-company");
      return "id" in res ? (res as CompanyDto) : null;
    },
  });
  const settings = useQuery({
    queryKey: ["company-settings", company.data?.id],
    queryFn: () =>
      api.get<CompanySettingsDto>(`/v1/companies/${company.data!.id}/settings`),
    enabled: !!company.data,
  });

  const [error, setError] = useState<string | null>(null);
  const cid = company.data?.id ?? 0;

  function invalidate() {
    void qc.invalidateQueries({ queryKey: ["company-settings"] });
  }

  function useAdd(path: string) {
    return useMutation({
      mutationFn: (body: Record<string, unknown>) => api.post(path, body),
      onSuccess: invalidate,
      onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
    });
  }

  const location = useAdd(`/v1/companies/${cid}/locations`);
  const series = useAdd(`/v1/companies/${cid}/document-series`);
  const owner = useAdd(`/v1/companies/${cid}/beneficial-owners`);
  const parent = useAdd(`/v1/companies/${cid}/ultimate-parents`);

  const s = settings.data;

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Настройки на фирмата</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {settings.isLoading ? <p className="text-sm text-muted-foreground">Зареждане…</p> : null}

      <Section title={`Обекти (${s?.locations.length ?? 0})`}>
        <form
          onSubmit={(e: FormEvent<HTMLFormElement>) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            location.mutate({
              name: f.get("name"),
              location_type: f.get("type"),
              city: f.get("city"),
            });
          }}
          className="mb-4 flex flex-wrap items-end gap-3"
        >
          <div className="flex min-w-48 flex-col gap-1">
            <Label>Име</Label>
            <Input name="name" required />
          </div>
          <div className="flex flex-col gap-1">
            <Label>Тип</Label>
            <select
              name="type"
              className="h-9 rounded-md border bg-background px-2 text-sm"
            >
              <option value="OFFICE">офис</option>
              <option value="STORE">магазин</option>
              <option value="WAREHOUSE">склад</option>
              <option value="BRANCH">клон</option>
              <option value="OTHER">друго</option>
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <Label>Град</Label>
            <Input name="city" className="w-32" />
          </div>
          <Button type="submit" disabled={location.isPending}>
            Добави
          </Button>
        </form>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Име</TableHead>
              <TableHead>Тип</TableHead>
              <TableHead>Град</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {s?.locations.map((l) => (
              <TableRow key={l.id}>
                <TableCell>{l.name}</TableCell>
                <TableCell>{l.location_type}</TableCell>
                <TableCell>{l.city}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Section>

      <Section title={`Серии документи (${s?.document_series.length ?? 0})`}>
        <form
          onSubmit={(e: FormEvent<HTMLFormElement>) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            series.mutate({
              name: f.get("name"),
              document_types: f.get("types"),
              start_number: Number(f.get("start")) || 1,
            });
          }}
          className="mb-4 flex flex-wrap items-end gap-3"
        >
          <div className="flex min-w-48 flex-col gap-1">
            <Label>Име</Label>
            <Input name="name" required />
          </div>
          <div className="flex min-w-48 flex-col gap-1">
            <Label>Типове (със запетая)</Label>
            <Input name="types" />
          </div>
          <div className="flex flex-col gap-1">
            <Label>Първи номер</Label>
            <Input name="start" type="number" defaultValue={1} className="w-24" />
          </div>
          <Button type="submit" disabled={series.isPending}>
            Добави
          </Button>
        </form>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Име</TableHead>
              <TableHead>Типове</TableHead>
              <TableHead>Следващ номер</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {s?.document_series.map((d) => (
              <TableRow key={d.id}>
                <TableCell>{d.name}</TableCell>
                <TableCell>{d.document_types}</TableCell>
                <TableCell>{d.next_number}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Section>

      <Section title={`Действителни собственици (${s?.beneficial_owners.length ?? 0})`}>
        <form
          onSubmit={(e: FormEvent<HTMLFormElement>) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            owner.mutate({
              first_name_bg: f.get("first"),
              last_name_bg: f.get("last"),
              egn: f.get("egn"),
              ownership_percentage: f.get("share"),
            });
          }}
          className="mb-4 flex flex-wrap items-end gap-3"
        >
          <div className="flex flex-col gap-1">
            <Label>Име</Label>
            <Input name="first" required />
          </div>
          <div className="flex flex-col gap-1">
            <Label>Фамилия</Label>
            <Input name="last" required />
          </div>
          <div className="flex flex-col gap-1">
            <Label>ЕГН</Label>
            <Input name="egn" className="w-28" />
          </div>
          <div className="flex flex-col gap-1">
            <Label>Дял %</Label>
            <Input name="share" className="w-20" />
          </div>
          <Button type="submit" disabled={owner.isPending}>
            Добави
          </Button>
        </form>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Име</TableHead>
              <TableHead>ЕГН</TableHead>
              <TableHead>Дял</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {s?.beneficial_owners.map((o) => (
              <TableRow key={o.id}>
                <TableCell>
                  {o.first_name_bg} {o.last_name_bg}
                </TableCell>
                <TableCell>{o.egn}</TableCell>
                <TableCell>{o.ownership_percentage}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Section>

      <Section title={`Крайни майки (${s?.ultimate_parents.length ?? 0})`}>
        <form
          onSubmit={(e: FormEvent<HTMLFormElement>) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            parent.mutate({ name_bg: f.get("name"), uic: f.get("uic") });
          }}
          className="mb-4 flex flex-wrap items-end gap-3"
        >
          <div className="flex min-w-52 flex-col gap-1">
            <Label>Име</Label>
            <Input name="name" required />
          </div>
          <div className="flex flex-col gap-1">
            <Label>УИК</Label>
            <Input name="uic" className="w-32" />
          </div>
          <Button type="submit" disabled={parent.isPending}>
            Добави
          </Button>
        </form>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Име</TableHead>
              <TableHead>УИК</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {s?.ultimate_parents.map((p) => (
              <TableRow key={p.id}>
                <TableCell>{p.name_bg}</TableCell>
                <TableCell>{p.uic}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </Section>
    </main>
  );
}
