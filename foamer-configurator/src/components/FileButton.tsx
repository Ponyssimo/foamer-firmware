import { useSelector } from "@tanstack/react-store";
import type { Config } from "../stores/configStore";
import { configStore } from "../stores/configStore";
import { errorStore } from "../stores/errorStore";

export default function FileButton() {
    const config = useSelector(configStore, (config) => config);
    const error = useSelector(errorStore, (error) => error);
    return (
        <details className="rounded-full border border-[var(--chip-line)] bg-[var(--chip-bg)] px-3 py-1.5 text-sm font-semibold text-[var(--sea-ink)] shadow-[0_8px_22px_rgba(30,90,72,0.08)] transition hover:-translate-y-0.5">
            <summary>File</summary>
            <div className="mt-2 min-w-56 rounded-xl border border-[var(--line)] bg-[var(--header-bg)] p-2 shadow-lg sm:absolute sm:right-0">
                {error ? (
                    <div className="text-red-600">Bad config, can't save</div>
                ) : (
                    <a
                        href={`data:application/json;charset=utf-8,${encodeURIComponent(JSON.stringify(config))}`}
                        download="config.json"
                        className="cursor-pointer block"
                    >
                        Save Config To File
                    </a>
                )}
                <label htmlFor="configFile" className="block cursor-pointer">
                    Load Config From File
                    <input
                        type="file"
                        id="configFile"
                        name="configFile"
                        className="hidden"
                        accept="application/json"
                        onChange={(event) => {
                            console.log("Event", event, event.target.files);
                            const file = event.target.files?.[0];
                            if (!file) {
                                console.log("No file, ignoring it");
                                return;
                            }
                            const reader = new FileReader();
                            reader.addEventListener("load", (_) => {
                                let config: Config;
                                try {
                                    config = JSON.parse(
                                        reader.result as string,
                                    );
                                } catch (err) {
                                    console.error("Bad config file!", err);
                                    alert(`Invalid config file... ${err}`);
                                    event.target.value = "";
                                    return;
                                }
                                event.target.value = "";
                                configStore.setState((_) => config);
                            });
                            console.log("Reading file", file);
                            reader.readAsText(file);
                        }}
                    />
                </label>
            </div>
        </details>
    );
}
