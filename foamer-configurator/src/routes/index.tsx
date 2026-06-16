import { useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import {
    configStore,
    BRAKE_START_INDEX,
    HORN_INDEX,
    TRIPLE_SWITCHES,
    TRIPLE_SWITCH_FUNCTION_COUNT,
    TRIPLE_SWITCH_START_INDEX,
} from "../stores/configStore";
import { useSelector } from "@tanstack/react-store";
import { AddressSelector } from "../components/AddressSelector";
import { FunctionGroup } from "../components/FunctionGroup";
import { ErrorBanner } from "../components/ErrorBanner";
import { WiThrottleConfig } from "../components/WiThrottleConfig";
//import {greet} from "../../pkg";

export const Route = createFileRoute("/")({ component: App });

function App() {
    const [profileId, setProfileId] = useState(0);
    const profile = useSelector(
        configStore,
        (config) => config.profiles[Number(profileId)],
    );
    const wifiConfig = useSelector(configStore, (config) => config.wifi_config);

    return (
        <main className="page-wrap px-4 pb-8 pt-14">
            <ErrorBanner />
            <section className="island-shell mt-8 rounded-2xl p-6">
                <p className="island-kicker mb-2">Profiles</p>
                <label
                    htmlFor="profile"
                    className="block text-sm font-semibold text-[var(--sea-ink)]"
                >
                    Profile
                    <select
                        name="profile"
                        id="profile"
                        value={profileId}
                        className="my-2 demo-select"
                        onChange={(event) => {
                            setProfileId(Number(event.target.value));
                        }}
                    >
                        {new Array(10).fill(null).map((_, key) => (
                            <option value={key} key={key}>
                                {key}
                            </option>
                        ))}
                    </select>
                </label>
                <AddressSelector
                    value={profile.address}
                    onChange={(value) =>
                        configStore.setState((config) => {
                            config = structuredClone(config);
                            config.profiles[profileId].address = value;
                            return config;
                        })
                    }
                />

                <FunctionGroup
                    groupName="Function Buttons"
                    typeLabel={(name) => `When ${name} is pressed`}
                    start={0}
                    profileId={profileId}
                    functions={new Array(4)
                        .fill(null)
                        .map((_, index) => `User ${index + 1}`)
                        .concat(["Bell", "Dynamics"])}
                />

                <FunctionGroup
                    groupName="Lights"
                    typeLabel={(_name, index) =>
                        `When ${TRIPLE_SWITCH_LABELS[Math.floor(index / TRIPLE_SWITCH_FUNCTION_COUNT)]} switch is in the ${TRIPLE_SWITCH_POSITION_LABELS[index % TRIPLE_SWITCH_FUNCTION_COUNT]} position`
                    }
                    start={TRIPLE_SWITCH_START_INDEX}
                    profileId={profileId}
                    functions={new Array(
                        TRIPLE_SWITCHES * TRIPLE_SWITCH_FUNCTION_COUNT,
                    )
                        .fill(null)
                        .map((_, index) => {
                            const positionId =
                                index % TRIPLE_SWITCH_FUNCTION_COUNT;
                            const switchId = Math.floor(
                                index / TRIPLE_SWITCH_FUNCTION_COUNT,
                            );
                            return `${TRIPLE_SWITCH_LABELS[switchId]} ${TRIPLE_SWITCH_POSITION_LABELS[positionId]}`;
                        })}
                />

                <FunctionGroup
                    groupName="Horn"
                    typeLabel={(_name, _index) =>
                        "When horn lever is held"
                    }
                    start={HORN_INDEX}
                    profileId={profileId}
                    functions={["Horn lever"]}
                />

                <FunctionGroup
                    groupName="Brake"
                    typeLabel={(name) => `When brake handle is in ${name}`}
                    start={BRAKE_START_INDEX}
                    profileId={profileId}
                    functions={BRAKE_LABELS}
                />
            </section>

            <section className="island-shell mt-8 rounded-2xl p-6">
                <p className="island-kicker mb-2">WiFi Configuration</p>
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
                            configStore.setState((config) => {
                                config = structuredClone(config);
                                config.wifi_config.ssid = event.target.value;
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
                            configStore.setState((config) => {
                                config = structuredClone(config);
                                config.wifi_config.password =
                                    event.target.value || null;
                                return config;
                            });
                        }}
                    />
                </label>
            </section>
            <WiThrottleConfig />
        </main>
    );
}

const TRIPLE_SWITCH_LABELS: [string, string, string] = [
    "Ditch Lights",
    "Rear Headlights",
    "Front Headlights",
];

const TRIPLE_SWITCH_POSITION_LABELS: [string, string, string] = [
    "Up",
    "Middle",
    "Down",
];

const BRAKE_LABELS: [string, string, string, string, string] = [
    "Brake Released Position",
    "Brake Step 1",
    "Brake Step 2",
    "Brake Step 3",
    "Emergency Brake Position",
];
