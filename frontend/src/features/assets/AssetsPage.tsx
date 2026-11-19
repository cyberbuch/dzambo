import { useState } from "react";

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
import type { DepRowDto } from "@/bindings/DepRowDto";

interface AssetRow {
  id: number;
  name: string;
  inventory_number: string;
  acquisition_date: string;
  cost: string;
  useful_life_months: number;
  accumulated_depreciation: string;
  status: string;
}

export default function AssetsPage() {
  const qc = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [period, setPeriod] = useState(() => new Date().toISOString().slice(0, 7));
  const [name, setName] = useState("");
  const [cost, setCost] = useState("0");
  const [life, setLife] = useState("60");

  const list = useQuery({
    queryKey: ["fixed-assets"],
    queryFn: () => api.get<AssetRow[]>("/v1/fixed-assets"),
  });
  const preview = useQuery({
    queryKey: ["dep-preview", period],
    queryFn: () =>
      api.post<DepRowDto[]>("/v1/fixed-assets/depreciation/preview", {
        year: Number(period.slice(0, 4)),
        month: Number(period.slice(5, 7)),
      }),
  });

  const create = useMutation({
    mutationFn: () =>
      api.post("/v1/fixed-assets", {
        name,
        acquisition_date: new Date().toISOString().slice(0, 10),
        cost,
        salvage_value: "0",
        useful_life_months: Number(life) || 0,
      }),
    onSuccess: () => {
      setName("");
      setMessage("активът е добавен");
      void qc.invalidateQueries({ queryKey: ["fixed-assets"] });
      void qc.invalidateQueries({ queryKey: ["dep-preview"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const post = useMutation({
    mutationFn: () =>
      api.post<{ entry_id: number }>("/v1/fixed-assets/depreciation/post", {
        year: Number(period.slice(0, 4)),
        month: Number(period.slice(5, 7)),
      }),
    onSuccess: (d) => setMessage(`амортизацията е записана (статия ${d.entry_id})`),
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">ДМА</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {message ? <p className="text-sm text-muted-foreground">{message}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Нов актив</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-52 flex-col gap-1">
              <Label>Име</Label>
              <Input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="flex w-32 flex-col gap-1">
              <Label>Цена</Label>
              <Input value={cost} onChange={(e) => setCost(e.target.value)} />
            </div>
            <div className="flex w-28 flex-col gap-1">
              <Label>Полезен живот (мес.)</Label>
              <Input value={life} onChange={(e) => setLife(e.target.value)} />
            </div>
            <Button disabled={!name || create.isPending} onClick={() => create.mutate()}>
              Добави
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Име</TableHead>
                <TableHead>Инв. №</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead className="text-right">Цена</TableHead>
                <TableHead>Живот</TableHead>
                <TableHead className="text-right">Намаление</TableHead>
                <TableHead>Статус</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(list.data ?? []).map((a) => (
                <TableRow key={a.id}>
                  <TableCell>{a.name}</TableCell>
                  <TableCell>{a.inventory_number}</TableCell>
                  <TableCell>{a.acquisition_date}</TableCell>
                  <TableCell className="text-right">{a.cost}</TableCell>
                  <TableCell>{a.useful_life_months}</TableCell>
                  <TableCell className="text-right">{a.accumulated_depreciation}</TableCell>
                  <TableCell>{a.status}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Месечна амортизация</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <Label>Месец</Label>
              <Input type="month" value={period} onChange={(e) => setPeriod(e.target.value)} className="w-36" />
            </div>
            <Button variant="outline" onClick={() => void preview.refetch()}>Преглед</Button>
            <Button
              variant="destructive"
              disabled={post.isPending || !(preview.data?.length)}
              onClick={() => post.mutate()}
            >
              Начисли амортизацията
            </Button>
          </div>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Актив</TableHead>
                <TableHead className="text-right">Сума</TableHead>
                <TableHead>Разход</TableHead>
                <TableHead>Отпис</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(preview.data ?? []).map((r) => (
                <TableRow key={r.asset_id}>
                  <TableCell>{r.name}</TableCell>
                  <TableCell className="text-right">{r.amount}</TableCell>
                  <TableCell>{r.expense_account}</TableCell>
                  <TableCell>{r.depreciation_account}</TableCell>
                </TableRow>
              ))}
              {!(preview.data?.length) ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground">
                    Няма амортизация за месеца.
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
