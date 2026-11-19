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
import type { CounterpartDto } from "@/bindings/CounterpartDto";
import type { Paged } from "@/bindings/Paged";

export default function CounterpartsPage() {
  const qc = useQueryClient();
  const list = useQuery({
    queryKey: ["counterparts"],
    queryFn: () => api.get<Paged<CounterpartDto>>("/v1/counterparts?limit=500"),
  });
  const [filter, setFilter] = useState("");
  const [name, setName] = useState("");
  const [type, setType] = useState("both");
  const [eik, setEik] = useState("");
  const [vat, setVat] = useState("");
  const [city, setCity] = useState("");
  const [error, setError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: () =>
      api.post<CounterpartDto>("/v1/counterparts", {
        name: name.trim(),
        counterpart_type: type,
        eik: eik.trim(),
        vat_number: vat.trim(),
        city: city.trim(),
      }),
    onSuccess: () => {
      setName("");
      setEik("");
      setVat("");
      setCity("");
      setError(null);
      void qc.invalidateQueries({ queryKey: ["counterparts"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  function submit(e: FormEvent) {
    e.preventDefault();
    create.mutate();
  }

  const items = (list.data?.items ?? []).filter((c) =>
    c.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Контрагенти</h1>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Нов контрагент
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-52 flex-col gap-1">
              <Label htmlFor="cp-name">Име</Label>
              <Input
                id="cp-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="cp-type">Вид</Label>
              <select
                id="cp-type"
                value={type}
                onChange={(e) => setType(e.target.value)}
                className="h-9 rounded-md border bg-background px-2 text-sm"
              >
                <option value="customer">клиент</option>
                <option value="supplier">доставчик</option>
                <option value="both">двустранен</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="cp-eik">ЕИК</Label>
              <Input
                id="cp-eik"
                value={eik}
                onChange={(e) => setEik(e.target.value)}
                className="w-32"
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="cp-vat">ДДС номер</Label>
              <Input
                id="cp-vat"
                value={vat}
                onChange={(e) => setVat(e.target.value)}
                className="w-36"
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="cp-city">Град</Label>
              <Input
                id="cp-city"
                value={city}
                onChange={(e) => setCity(e.target.value)}
                className="w-32"
              />
            </div>
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
            Контрагенти ({items.length})
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
                <TableHead>Име</TableHead>
                <TableHead>Вид</TableHead>
                <TableHead>ЕИК</TableHead>
                <TableHead>ДДС</TableHead>
                <TableHead>Град</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((c) => (
                <TableRow key={c.id}>
                  <TableCell>{c.name}</TableCell>
                  <TableCell>{c.counterpart_type}</TableCell>
                  <TableCell>{c.eik}</TableCell>
                  <TableCell>{c.vat_number}</TableCell>
                  <TableCell>{c.city}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
