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
import type { OrderDto } from "@/bindings/OrderDto";
import type { ProductDto } from "@/bindings/ProductDto";
import type { Paged } from "@/bindings/Paged";
import type { RecipeDto } from "@/bindings/RecipeDto";

export default function ProductionPage() {
  const qc = useQueryClient();
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [rName, setRName] = useState("");
  const [outProduct, setOutProduct] = useState("");
  const [matProduct, setMatProduct] = useState("");
  const [matQty, setMatQty] = useState("1");
  const [orderRecipe, setOrderRecipe] = useState("");
  const [orderQty, setOrderQty] = useState("1");

  const recipes = useQuery({
    queryKey: ["recipes"],
    queryFn: () => api.get<RecipeDto[]>("/v1/recipes"),
  });
  const orders = useQuery({
    queryKey: ["production-orders"],
    queryFn: () => api.get<OrderDto[]>("/v1/production-orders"),
  });
  const products = useQuery({
    queryKey: ["products"],
    queryFn: () => api.get<Paged<ProductDto>>("/v1/products?limit=500"),
  });

  const refresh = () => {
    void qc.invalidateQueries({ queryKey: ["recipes"] });
    void qc.invalidateQueries({ queryKey: ["production-orders"] });
    void qc.invalidateQueries({ queryKey: ["stock-report"] });
  };

  const createRecipe = useMutation({
    mutationFn: () =>
      api.post("/v1/recipes", {
        name: rName,
        output_product_id: Number(outProduct),
        output_quantity: "1",
        lines: [{ product_id: Number(matProduct), quantity: matQty, wastage_percent: "0" }],
      }),
    onSuccess: () => {
      setRName("");
      setMessage("рецептата е добавена");
      refresh();
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const createOrder = useMutation({
    mutationFn: () =>
      api.post("/v1/production-orders", {
        recipe_id: Number(orderRecipe),
        quantity: orderQty,
        order_date: new Date().toISOString().slice(0, 10),
      }),
    onSuccess: () => {
      setMessage("нареждането е създадено");
      refresh();
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const confirm = useMutation({
    mutationFn: (id: number) =>
      api.post<{ material_cost: string }>(`/v1/production-orders/${id}/confirm`),
    onSuccess: (d) => {
      setMessage(`материали: ${d.material_cost}`);
      refresh();
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  const prodOption = (p: ProductDto) => `${p.code} ${p.name}`;

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Производство</h1>
      {error ? <p className="text-sm text-destructive">{error}</p> : null}
      {message ? <p className="text-sm text-muted-foreground">{message}</p> : null}

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Нова рецепта</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-48 flex-col gap-1">
              <Label>Име</Label>
              <Input value={rName} onChange={(e) => setRName(e.target.value)} />
            </div>
            <div className="flex min-w-52 flex-col gap-1">
              <Label>Изходяща стока</Label>
              <select value={outProduct} onChange={(e) => setOutProduct(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(products.data?.items ?? []).map((p) => (
                  <option key={p.id} value={p.id}>{prodOption(p)}</option>
                ))}
              </select>
            </div>
            <div className="flex min-w-52 flex-col gap-1">
              <Label>Материал</Label>
              <select value={matProduct} onChange={(e) => setMatProduct(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(products.data?.items ?? []).map((p) => (
                  <option key={p.id} value={p.id}>{prodOption(p)}</option>
                ))}
              </select>
            </div>
            <div className="flex w-24 flex-col gap-1">
              <Label>Кол. за 1 бр.</Label>
              <Input value={matQty} onChange={(e) => setMatQty(e.target.value)} />
            </div>
            <Button disabled={!rName || !outProduct || !matProduct || createRecipe.isPending} onClick={() => createRecipe.mutate()}>
              Добави
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Ново нареждане</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-52 flex-col gap-1">
              <Label>Рецепта</Label>
              <select value={orderRecipe} onChange={(e) => setOrderRecipe(e.target.value)} className="h-9 rounded-md border bg-background px-2 text-sm">
                <option value="">—</option>
                {(recipes.data ?? []).map((r) => (
                  <option key={r.id} value={r.id}>{r.name}</option>
                ))}
              </select>
            </div>
            <div className="flex w-28 flex-col gap-1">
              <Label>Количество</Label>
              <Input value={orderQty} onChange={(e) => setOrderQty(e.target.value)} />
            </div>
            <Button disabled={!orderRecipe || createOrder.isPending} onClick={() => createOrder.mutate()}>
              Създай
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-4">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Номер</TableHead>
                <TableHead>Дата</TableHead>
                <TableHead>Статус</TableHead>
                <TableHead className="text-right">Количество</TableHead>
                <TableHead className="text-right">Материали</TableHead>
                <TableHead></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {(orders.data ?? []).map((o) => (
                <TableRow key={o.id}>
                  <TableCell>{o.number || `№${o.id}`}</TableCell>
                  <TableCell>{o.order_date}</TableCell>
                  <TableCell>{o.status}</TableCell>
                  <TableCell className="text-right">{o.quantity}</TableCell>
                  <TableCell className="text-right">{o.material_cost}</TableCell>
                  <TableCell>
                    {o.status === "draft" ? (
                      <Button size="sm" onClick={() => confirm.mutate(o.id)}>Потвърди</Button>
                    ) : null}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
