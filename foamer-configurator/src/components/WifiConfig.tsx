import { setConfig, useConfig, WIFI_COUNT } from "../stores/configStore";

export function WifiConfig() {
    const wifiConfigs = useConfig((config) => config.base_config.wifi_configs);

    return (
        <div>
            {wifiConfigs.map((_, wifiIndex) => (
                <div
                    key={wifiIndex}
                    className="flex flex-col md:flex-row md:gap-4"
                >
                    <div className="flex-1">
                        <WifiConfigEntry wifiIndex={wifiIndex} />
                    </div>
                    <div className="flex justify-end flex-col">
                        <label>
                            <button
                                type="button"
                                className="demo-button-input text-sm my-2"
                                onClick={() => {
                                    setConfig((config) => {
                                        config = structuredClone(config);
                                        config.base_config.wifi_configs.splice(
                                            wifiIndex,
                                            1,
                                        );
                                        return config;
                                    });
                                }}
                                disabled={wifiConfigs.length <= 1}
                            >
                                Remove
                            </button>
                        </label>
                    </div>
                </div>
            ))}
            <button
                type="button"
                className="demo-button"
                onClick={() => {
                    setConfig((config) => {
                        config = structuredClone(config);
                        config.base_config.wifi_configs.push({
                            ssid: "",
                            password: null,
                        });
                        return config;
                    });
                }}
                disabled={wifiConfigs.length >= WIFI_COUNT}
            >
                Add Network
            </button>
        </div>
    );
}

export function WifiConfigEntry({ wifiIndex }: { wifiIndex: number }) {
    const wifiConfig = useConfig(
        (config) => config.base_config.wifi_configs[wifiIndex],
    );

    return (
        <>
            <label
                htmlFor="ssid"
                className="block text-sm font-semibold text-[var(--sea-ink)]"
            >
                WiFi SSID
                <input
                    name="ssid"
                    id="ssid"
                    type="text"
                    className="my-2 demo-input"
                    value={wifiConfig.ssid}
                    onChange={(event) => {
                        setConfig((config) => {
                            config = structuredClone(config);
                            config.base_config.wifi_configs[wifiIndex].ssid =
                                event.target.value;
                            return config;
                        });
                    }}
                />
            </label>

            <label
                htmlFor="ssid"
                className="block text-sm font-semibold text-[var(--sea-ink)]"
            >
                WiFi Password
                <input
                    name="ssid"
                    id="ssid"
                    type="password"
                    className="my-2 demo-input"
                    value={wifiConfig.password ?? ""}
                    onChange={(event) => {
                        setConfig((config) => {
                            config = structuredClone(config);
                            config.base_config.wifi_configs[
                                wifiIndex
                            ].password = event.target.value || null;
                            return config;
                        });
                    }}
                />
            </label>
        </>
    );
}
