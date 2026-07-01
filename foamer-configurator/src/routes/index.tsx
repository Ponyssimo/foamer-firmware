import { createFileRoute } from "@tanstack/react-router";
import { useSelector } from "@tanstack/react-store";
import { useState } from "react";
import { Consist } from "../components/Consist";
import { ErrorBanner } from "../components/ErrorBanner";
import { FunctionGroup } from "../components/FunctionGroup";
import { WifiConfig } from "../components/WifiConfig";
import { WiThrottleConfig } from "../components/WiThrottleConfig";
import {
    BRAKE_START_INDEX,
    configStore,
    DEFAULT_CONFIG,
    HORN_INDEX,
    TRIPLE_SWITCH_FUNCTION_COUNT,
    TRIPLE_SWITCH_START_INDEX,
    TRIPLE_SWITCHES,
} from "../stores/configStore";
//import {greet} from "../../pkg";

export const Route = createFileRoute("/")({ component: App });

function App() {
    const [profileId, setProfileId] = useState(0);
    const configStoreValue = useSelector(
        configStore,
        (storeValue) => storeValue,
    );

    return (
        <main className="page-wrap px-4 pb-8 pt-14">
            <ErrorBanner />
            {configStoreValue.type == "ParsingError" && (
                <div>
                    <p className="m-0 max-w-3xl text-base leading-8 text-[var(--sea-ink-soft)] my-2">
                        Stored config is invalid.
                    </p>
                    <button
                        type="button"
                        className="demo-button"
                        onClick={() => {
                            configStore.setState((_) => ({
                                type: "Config",
                                data: structuredClone(DEFAULT_CONFIG),
                            }));
                        }}
                    >
                        Load default config
                    </button>
                    <a
                        href={`data:application/json;charset=utf-8,${encodeURIComponent(configStoreValue.json)}`}
                        download="config_broken.json"
                        className="demo-button"
                    >
                        Save Existing Config To File
                    </a>
                </div>
            )}
            {configStoreValue.type == "Config" && (
                <>
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
                    </section>
                    <section className="island-shell mt-8 rounded-2xl p-6">
                        <p className="island-kicker mb-2">Consist</p>
                        <p className="m-0 max-w-3xl text-base leading-8 text-[var(--sea-ink-soft)] my-2">
                            Which locomotives should be controlled by this
                            profile?
                        </p>

                        <Consist profileId={profileId} />
                    </section>

                    <section className="island-shell mt-8 rounded-2xl p-6">
                        <p className="island-kicker mb-2">Inputs</p>
                        <p className="m-0 max-w-3xl text-base leading-8 text-[var(--sea-ink-soft)] my-2">
                            What should happen when buttons are pressed?
                        </p>

                        <div className="flex gap-8 flex-col">
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
                                    TRIPLE_SWITCHES *
                                        TRIPLE_SWITCH_FUNCTION_COUNT,
                                )
                                    .fill(null)
                                    .map((_, index) => {
                                        const positionId =
                                            index %
                                            TRIPLE_SWITCH_FUNCTION_COUNT;
                                        const switchId = Math.floor(
                                            index /
                                                TRIPLE_SWITCH_FUNCTION_COUNT,
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
                                typeLabel={(name) =>
                                    `When brake handle is in ${name}`
                                }
                                start={BRAKE_START_INDEX}
                                profileId={profileId}
                                functions={BRAKE_LABELS}
                            />
                        </div>
                    </section>

                    <section className="island-shell mt-8 rounded-2xl p-6">
                        <p className="island-kicker mb-2">WiFi Configuration</p>

                        <p className="m-0 max-w-3xl text-base leading-8 text-[var(--sea-ink-soft)] my-2">
                            What WiFi network should be used?
                        </p>

                        <WifiConfig />
                    </section>
                    <WiThrottleConfig />
                </>
            )}
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
