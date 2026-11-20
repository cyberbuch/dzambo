# dzambo Next.js фронт — standalone build.
# YTBUCH_API_PROXY се задава при build (rewrites се bake-ват в standalone).

FROM node:22-bookworm AS builder
WORKDIR /app
ARG YTBUCH_API_PROXY=http://127.0.0.1:8095
ENV YTBUCH_API_PROXY=$YTBUCH_API_PROXY
COPY package.json package-lock.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM node:22-bookworm-slim AS runtime
WORKDIR /app
ENV NODE_ENV=production HOSTNAME=0.0.0.0 PORT=3000
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public
EXPOSE 3000
CMD ["node", "server.js"]