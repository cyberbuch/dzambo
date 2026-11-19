import { useState } from "react";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { api, ApiError } from "@/lib/api";
import type { ProductDto } from "@/bindings/ProductDto";
import type { Paged } from "@/bindings/Paged";
import type { StockDocDto } from "@/bindings/StockDocDto";
import type { StockItemDto } from "@/bindings/StockItemDto";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export default function StockPage() {
  const qc = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [kind, setKind] = useState("issue");
  const [productId, setProductId] = useState("");
  const [quantity, setQuantity] = useState("1");

  const report = useQuery({
    queryKey: ["stock-report"],
    queryFn: () => api.get<StockItemDto[]>("/v1/stock/report"),
  });
  const products = useQuery({
    queryKey: ["products"],
    queryFn: () => api.get<Paged<ProductDto>>("/v1/products?limit=500"),
  });
  const docs = useQuery({
    queryKey: ["stock-docs"],
    queryFn: () => api.get<StockDocDto[]>("/v1/stock-documents"),
  });

  const createDoc = useMutation({
    mutationFn: () =>
      api.post("/v1/stock-documents", {
        doc_date: new Date().toISOString().slice(0, 10),
        kind,
        notes: "",
        lines: [{ product_id: Number(productId), quantity }],
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["stock-report"] });
      void qc.invalidateQueries({ queryKey: ["stock-docs"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const total = (report.data ?? []).reduce((s, r) => s + Number(r.value), 0);

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Склад</h1>

      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            Наличност по партиди (WAC)
          </CardTitle>
          <span className="text-sm font-medium">Обща стойност: {total.toFixed(2)}</span>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Стока</TableHead>
                <TableHead>Код</TableHead>
                <TableHead>Ед.</TableHead>
                <TableHead className="text-right">Количество</TableHead>
                <TableHead className="text-right">Ср. цена</TableHead>
                <TableHead className="text-right">Стойност</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(report.data ?? []).map((r) => (
                <TableRow key={r.product_id}>
                  <TableCell>{r.name}</TableCell>
                  <TableCell>{r.code}</TableCell>
                  <TableCell>{r.unit}</TableCell>
                  <TableCell className="text-right">{r.quantity}</TableCell>
                  <TableCell className="text-right">{r.avg_unit_cost}</TableCell>
                  <TableCell className="text-right">{r.value}</TableCell>
                </TableRow>
              ))}
              {!(report.data?.length) ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-muted-foreground">
                    Няма наличности — закупени стоки първо стават партиди.
                  </TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Ръчен документ</CardTitle>
        </CardHeader>
        <CardContent>
          {error ? <p className="mb-3 text-sm text-destructive">{error}</p> : null}
          <div className="mb-4 flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <Label>Вид</Label>
              <select value={kind} onChange={(e) => setKind(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="issue">нужда</option>
                <option value="writeoff">списание</option>
                <option value="move">движение</option>
              </select>
            </div>
            <div className="flex min-w-56 flex-col gap-1">
              <Label>Стока</Label>
              <select value={productId} onChange={(e) => setProductId(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(products.data?.items ?? []).map((p) => (
                  <option key={p.id} value={p.id}>{p.code} {p.name}</option>
                ))}
              </select>
            </div>
            <div className="flex w-28 flex-col gap-1">
              <Label>Количество</Label>
              <Input value={quantity} onChange={(e) => setQuantity(e.target.value)} />
            </div>
            <Button disabled={!productId || createDoc.isPending} onClick={() => createDoc.mutate()}>Създай</Button>
          </div>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead>Вид</TableHead>
                <TableHead>Стокa</TableHead>
                <TableHead className="text-right">Кол.</TableHead>
                <TableHead className="text-right">Ед. цена</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(docs.data ?? []).map((d) => d.lines.map((l, i) => (
                <TableRow key={`${d.id}-${i}`}>
                  <TableCell>{d.number}</TableCell>
                  <TableCell>{d.doc_date}</TableCell>
                  <TableCell>{d.kind}</TableCell>
                  <TableCell>{l.product_id}</TableCell>
                  <TableCell className="text-right">{l.quantity}</TableCell>
                  <TableCell className="text-right">{l.unit_cost}</TableCell>
                </TableRow>
              )))}
              {!(docs.data?.length) ? (
                <TableRow><TableCell colSpan={6} className="text-muted-foreground">Няма ръчни документи.</TableCell></TableRow>
              ) : null}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
