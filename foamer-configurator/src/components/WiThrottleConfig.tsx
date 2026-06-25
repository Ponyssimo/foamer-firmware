import { useSelector } from "@tanstack/react-store";
import { configStore } from "../stores/configStore";
import type { WiThrottleDiscovery } from "../stores/configStore";

function isHardcoded(
    discovery: WiThrottleDiscovery,
): discovery is { Hardcoded: string } {
    return typeof discovery == "object" && "Hardcoded" in discovery;
}

export function WiThrottleConfig() {
    const discoveryConfig = useSelector(
        configStore,
        (config) => config.base_config.withrottle_server.discovery,
    );
    const discoveryType = discoveryConfig == "Mdns" ? "Mdns" : "Hardcoded";
    return (
        <section className="island-shell mt-8 rounded-2xl p-6">
            <p className="island-kicker mb-2">
                WiThrottle Server Configuration
            </p>
            <label
                htmlFor="discovery"
                className="block text-sm font-semibold text-[var(--sea-ink)]"
            >
                WiThrottle Server Discovery Method
                <select
                    name="discovery"
                    id="discovery"
                    value={discoveryType}
                    className="my-2 demo-select"
                    onChange={(event) => {
                        configStore.setState((config) => {
                            config = structuredClone(config);
                            if (event.target.value == "Hardcoded") {
                                config.base_config.withrottle_server.discovery = {
                                    Hardcoded: "192.0.2.69:12090",
                                };
                            } else if (event.target.value == "Mdns") {
                                config.base_config.withrottle_server.discovery = "Mdns";
                            }
                            return config;
                        });
                    }}
                >
                    {["Mdns", "Hardcoded"].map((key) => (
                        <option value={key} key={key}>
                            {key}
                        </option>
                    ))}
                </select>
            </label>
            {isHardcoded(discoveryConfig) && (
                <label
                    htmlFor="serverAddress"
                    className="block text-sm font-semibold text-[var(--sea-ink)]"
                >
                    <input
                        name="serverAddress"
                        id="serverAddress"
                        type="text"
                        className="my-2 demo-input"
                        value={discoveryConfig.Hardcoded}
                        onChange={(event) => {
                            configStore.setState((config) => {
                                config = structuredClone(config);
                                config.base_config.withrottle_server.discovery = {
                                    Hardcoded: event.target.value,
                                };
                                return config;
                            });
                        }}
                    />
                </label>
            )}
        </section>
    );
}
