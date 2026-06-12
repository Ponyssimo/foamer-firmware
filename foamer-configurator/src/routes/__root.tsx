import {useEffect} from "react";
import {configStore} from "../stores/configStore";
import {errorStore} from "../stores/errorStore";
import {TanStackDevtools} from "@tanstack/react-devtools";
import {createRootRoute, Scripts} from "@tanstack/react-router";
import {TanStackRouterDevtoolsPanel} from "@tanstack/react-router-devtools";
import Footer from "../components/Footer";
import Header from "../components/Header";
import type {Config} from "../stores/configStore";
import {wasmPromise} from "../wasm";

import appCss from "../styles.css?url";

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: "utf-8",
      },
      {
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      },
      {
        title: "TanStack Start Starter",
      },
    ],
    links: [
      {
        rel: "stylesheet",
        href: appCss,
      },
    ],
  }),
  shellComponent: RootDocument,
});

function RootDocument({children}: {children: React.ReactNode}) {
  useEffect(() => {
    if (!import.meta.env.SSR) {
      const localConfig = localStorage.getItem("config");
      let initialConfig: Config | undefined;
      if (localConfig) {
        try {
          initialConfig = JSON.parse(localConfig);
        } catch (err) {
          console.error("Failed to parse saved config", err);
        }
      }
      if (initialConfig) {
        configStore.setState((_) => initialConfig);
      }

      const subscription = configStore.subscribe((config) => {
        wasmPromise.then((wasm) => {
          const configString = JSON.stringify(config);
          try {
            wasm.encode_config(configString);
          } catch (err: string) {
            if (typeof err != "string") {
              throw err;
            }
            errorStore.setState((_) => err);
            return;
          }
          errorStore.setState((_) => null);

          localStorage.setItem("config", configString);
        });
      });
      return () => {
        subscription.unsubscribe();
      };
    }
  }, []);

  return (
    <div className="font-sans antialiased [overflow-wrap:anywhere] selection:bg-[rgba(79,184,178,0.24)]">
      <Header />
      {children}
      <Footer />
      <TanStackDevtools
        config={{
          position: "bottom-right",
        }}
        plugins={[
          {
            name: "Tanstack Router",
            render: <TanStackRouterDevtoolsPanel />,
          },
        ]}
      />
      <Scripts />
    </div>
  );
}
