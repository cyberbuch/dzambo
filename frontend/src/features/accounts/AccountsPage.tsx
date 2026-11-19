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
import type { AccountDto } from "@/bindings/AccountDto";
import type { Paged } from "@/bindings/Paged";

const ANALYTIC = ["none", "counterpart", "product"];

export default function AccountsPage() {
  const qc = useQueryClient();
  const list = useQuery({
    queryKey: ["accounts"],
    queryFn: () => api.get<Paged<AccountDto>>("/v1/accounts?limit=500"),
  });
  const [filter, setFilter] = useState("");
  const [number, setNumber] = useState("");
  const [name, setName] = useState("");
  const [analytic, setAnalytic] = useState("none");
  const [isCurrency, setIsCurrency] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: () =>
      api.post<AccountDto>("/v1/accounts", {
        number: number.trim(),
        name: name.trim(),
        saft_account_type: number.trim(),
        analytic_type: analytic,
        is_currency: isCurrency,
      }),
    onSuccess: () => {
      setNumber("");
      setName("");
      setError(null);
      void qc.invalidateQueries({ queryKey: ["accounts"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  function submit(e: FormEvent) {
    e.preventDefault();
    create.mutate();
  }

  const items = (list.data?.items ?? []).filter(
    (a) =>
      a.number.includes(filter.trim()) ||
      a.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Сметкоплан</h1>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Нова сметка
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <Label htmlFor="acc-number">Номер</Label>
              <Input
                id="acc-number"
                value={number}
                onChange={(e) => setNumber(e.target.value)}
                className="w-28"
                required
              />
            </div>
            <div className="flex min-w-52 flex-col gap-1">
              <Label htmlFor="acc-name">Име</Label>
              <Input
                id="acc-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="acc-analytic">Аналитичност</Label>
              <select
                id="acc-analytic"
                value={analytic}
                onChange={(e) => setAnalytic(e.target.value)}
                className="h-9 rounded-md border bg-background px-2 text-sm"
              >
                {ANALYTIC.map((t) => (
                  <option key={t} value={t}>
                    {t}
                  </option>
                ))}
              </select>
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={isCurrency}
                onChange={(e) => setIsCurrency(e.target.checked)}
              />
              Валутна
            </label>
            <Button type="submit" disabled={create.isPending}>
              Добави
            </Button>
            {error ? <span className="text-sm text-destructive">{error}</span> : null}
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Сметки ({items.length})
          </CardTitle>
          <Input
            placeholder="търсене…"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            className="w-56"
          />
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Име</TableHead>
                <TableHead>Аналитичност</TableHead>
                <TableHead>Валутна</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((a) => (
                <TableRow key={a.id}>
                  <TableCell>{a.number}</TableCell>
                  <TableCell>{a.name}</TableCell>
                  <TableCell>{a.analytic_type}</TableCell>
                  <TableCell>{a.is_currency ? "да" : ""}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
