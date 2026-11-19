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
import type { ProductDto } from "@/bindings/ProductDto";
import type { Paged } from "@/bindings/Paged";

const TYPES = [
  ["10", "на склад (10)"],
  ["20", "полуфабрикат (20)"],
  ["30", "материал (30)"],
  ["40", "готова продукция (40)"],
  ["50", "отпадък (50)"],
] as const;

export default function ProductsPage() {
  const qc = useQueryClient();
  const list = useQuery({
    queryKey: ["products"],
    queryFn: () => api.get<Paged<ProductDto>>("/v1/products?limit=500"),
  });
  const [filter, setFilter] = useState("");
  const [name, setName] = useState("");
  const [productType, setProductType] = useState("30");
  const [isInventory, setIsInventory] = useState(true);
  const [isService, setIsService] = useState(false);
  const [unit, setUnit] = useState("C62");
  const [price, setPrice] = useState("0");
  const [vatRate, setVatRate] = useState("20");
  const [error, setError] = useState<string | null>(null);

  const create = useMutation({
    mutationFn: () =>
      api.post<ProductDto>("/v1/products", {
        name: name.trim(),
        code: "",
        product_type: productType,
        is_inventory: isInventory,
        is_service: isService,
        unit,
        price,
        cost_price: "0",
        vat_rate: vatRate,
      }),
    onSuccess: () => {
      setName("");
      setError(null);
      void qc.invalidateQueries({ queryKey: ["products"] });
    },
    onError: (e) => setError(e instanceof ApiError ? e.message : String(e)),
  });

  function submit(e: FormEvent) {
    e.preventDefault();
    create.mutate();
  }

  const items = (list.data?.items ?? []).filter((p) =>
    p.name.toLowerCase().includes(filter.trim().toLowerCase()),
  );

  return (
    <main className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <h1 className="text-2xl font-semibold">Стоки</h1>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">Нова стока</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="flex flex-wrap items-end gap-3">
            <div className="flex min-w-52 flex-col gap-1">
              <Label htmlFor="pr-name">Име</Label>
              <Input
                id="pr-name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                required
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="pr-type">Вид запас</Label>
              <select
                id="pr-type"
                value={productType}
                onChange={(e) => setProductType(e.target.value)}
                className="h-9 rounded-md border bg-background px-2 text-sm"
              >
                {TYPES.map(([v, l]) => (
                  <option key={v} value={v}>
                    {l}
                  </option>
                ))}
              </select>
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={isInventory}
                onChange={(e) => setIsInventory(e.target.checked)}
              />
              Инвентарна
            </label>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={isService}
                onChange={(e) => setIsService(e.target.checked)}
              />
              Услуга
            </label>
            <div className="flex flex-col gap-1">
              <Label htmlFor="pr-unit">Мерна единица</Label>
              <Input
                id="pr-unit"
                value={unit}
                onChange={(e) => setUnit(e.target.value)}
                className="w-20"
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="pr-price">Цена</Label>
              <Input
                id="pr-price"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                className="w-24"
              />
            </div>
            <div className="flex flex-col gap-1">
              <Label htmlFor="pr-vat">ДДС %</Label>
              <Input
                id="pr-vat"
                value={vatRate}
                onChange={(e) => setVatRate(e.target.value)}
                className="w-20"
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
            Стоки ({items.length})
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
                <TableHead>Код</TableHead>
                <TableHead>Име</TableHead>
                <TableHead>Вид</TableHead>
                <TableHead>Ед.</TableHead>
                <TableHead className="text-right">Цена</TableHead>
                <TableHead className="text-right">ДДС</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.map((p) => (
                <TableRow key={p.id}>
                  <TableCell>{p.code}</TableCell>
                  <TableCell>{p.name}</TableCell>
                  <TableCell>{p.product_type || "услуга"}</TableCell>
                  <TableCell>{p.unit}</TableCell>
                  <TableCell className="text-right">{p.price ?? "—"}</TableCell>
                  <TableCell className="text-right">{p.vat_rate ?? "—"}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </main>
  );
}
