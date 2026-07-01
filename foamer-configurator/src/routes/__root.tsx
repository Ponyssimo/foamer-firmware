import { TanStackDevtools } from "@tanstack/react-devtools";
import { createRootRoute, Scripts } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { useEffect } from "react";
import Footer from "../components/Footer";
import Header from "../components/Header";
import { configStore, loadConfig } from "../stores/configStore";
import { errorStore } from "../stores/errorStore";
import appCss from "../styles.css?url";
import { wasmPromise } from "../wasm";

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

function RootDocument({ children }: { children: React.ReactNode }) {
    useEffect(() => {
        if (!import.meta.env.SSR) {
            const localConfig = localStorage.getItem("config");
            if (localConfig) {
                loadConfig(localConfig);
            }

            const subscription = configStore.subscribe((configStoreValue) => {
                console.log("New config store value", configStoreValue);
                if (configStoreValue.type != "Config") {
                    return;
                }
                const config = configStoreValue.data;
                console.log("Config", structuredClone(config));
                const configString = JSON.stringify(config);
                wasmPromise.then((wasm) => {
                    try {
                        wasm.encode_config(configString);
                    } catch (err) {
                        if (typeof err != "string") {
                            throw err;
                        }
                        errorStore.setState((_) => err);
                        return;
                    }
                    errorStore.setState((_) => null);
                });
                localStorage.config = configString;
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
