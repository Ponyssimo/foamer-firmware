FROM docker.io/rust:1.96-bookworm AS rust

RUN curl https://wasm-bindgen.github.io/wasm-pack/installer/init.sh -sSf | sh

COPY . /app/

WORKDIR /app/foamer-configurator

RUN wasm-pack build --release --panic-unwind

FROM docker.io/node:26-alpine3.24 as node

ENV CI=true
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN npm i -g corepack
RUN corepack enable

COPY --from=rust /app /app

WORKDIR /app/foamer-configurator

RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile

RUN NODE_ENV=production pnpm run build

FROM docker.io/nginxinc/nginx-unprivileged as serve
RUN sed -i 's/ root /try_files $uri $uri\/ \/_shell.html =404; root/' /etc/nginx/conf.d/default.conf
WORKDIR /app
COPY --from=node /app/foamer-configurator/dist /usr/share/nginx/html
