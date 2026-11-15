# API de comptes C02 Naval — aucune dépendance npm, Node >= 22.5 (SQLite intégré)
FROM node:24-alpine
WORKDIR /app
COPY server/ ./server/
ENV NODE_ENV=production PORT=8787 DB_PATH=/data/c02.sqlite
RUN mkdir -p /data && chown node:node /data
USER node
VOLUME /data
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD wget -qO- http://127.0.0.1:8787/api/health >/dev/null || exit 1
CMD ["node", "server/index.js"]
