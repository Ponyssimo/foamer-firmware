import type {
    Function,
    FunctionBehavior,
    FunctionConfig,
    FunctionType,
    Hardcoded,
    Label,
    Momentary,
} from "../stores/configStore";
import { setConfig, useConfig } from "../stores/configStore";

export function FunctionGroup({
    start,
    profileId,
    functions,
    typeLabel,
}: {
    profileId: number;
    groupName: string;
    start: number;
    functions: string[];
    typeLabel: (name: string, index: number) => string;
}) {
    return functions.map((name, index) => (
        <FunctionSwitch
            index={start + index}
            name={name}
            profileId={profileId}
            typeLabel={typeLabel(name, index)}
            key={start + index}
        />
    ));
}

const DEFAULT_FUNCTION_BY_TYPE: { [K in FunctionType]: Function } = {
    Label: {
        Label: {
            label: "",
            momentary: false,
        },
    },
    Hardcoded: {
        Hardcoded: {
            id: 0,
            momentary: false,
        },
    },
    EmergencyStop: "EmergencyStop",
};

function getFunctionType(
    func: Function | null | undefined,
): FunctionType | null {
    func = func ?? null;
    return (
        func &&
        ((func == "EmergencyStop" && func) ||
            ("Label" in func
                ? ("Label" as const)
                : "Hardcoded" in func
                  ? ("Hardcoded" as const)
                  : (func satisfies never)))
    );
}

function isHardcoded(func: Function | null | undefined): func is Hardcoded {
    return getFunctionType(func) == "Hardcoded";
}

function isLabel(func: Function | null | undefined): func is Label {
    return getFunctionType(func) == "Label";
}

// function isEmergencyStop(func: Function | null): func is "EmergencyStop" {
//     return getFunctionType(func) == "EmergencyStop";
// }
function getMomentary(func: Label | Hardcoded): Momentary {
    if (isLabel(func)) {
        return func.Label;
    } else if (isHardcoded(func)) {
        return func.Hardcoded;
    } else {
        return func satisfies never;
    }
}

function FunctionSwitch({
    index,
    name,
    profileId,
    typeLabel,
}: {
    index: number;
    name: string;
    profileId: number;
    typeLabel: string;
}) {
    const funcConfig = useConfig(
        (config) => config.profiles[profileId].functions[index],
    );

    const setFuncConfig = (
        sentinel: (func: FunctionConfig | null) => FunctionConfig | null,
    ) => {
        setConfig((config) => {
            config = structuredClone(config);
            const oldFunc = config.profiles[profileId].functions[index];
            const newFunc = sentinel(oldFunc);
            config.profiles[profileId].functions[index] = newFunc;
            return config;
        });
    };

    const id = `function-${index}`;
    return (
        <section className="island-shell rounded-2xl p-6">
            <p className="island-kicker mb-2">{name}</p>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <label
                    htmlFor={id}
                    className="block text-sm font-semibold text-[var(--sea-ink)]"
                >
                    {typeLabel}, activate:{" "}
                    <select
                        name={id}
                        id={id}
                        value={
                            getFunctionType(funcConfig?.function ?? null) ??
                            "null"
                        }
                        className="my-2 demo-select"
                        onChange={(event) => {
                            const value = event.target.value as
                                | FunctionType
                                | "null";
                            const defaultType =
                                value == "null"
                                    ? null
                                    : DEFAULT_FUNCTION_BY_TYPE[value];
                            setFuncConfig((funcConfig) => {
                                if (defaultType) {
                                    if (!funcConfig) {
                                        funcConfig = {
                                            behavior: "All",
                                            function: null as never,
                                        };
                                    }
                                    funcConfig.function =
                                        structuredClone(defaultType);
                                } else {
                                    funcConfig = null;
                                }
                                return funcConfig;
                            });
                        }}
                    >
                        {(
                            [
                                "null",
                                "Label",
                                "Hardcoded",
                                "EmergencyStop",
                            ] as const
                        ).map((name) => (
                            <option value={name} key={name}>
                                {NICE_LABELS[name]}
                            </option>
                        ))}
                    </select>
                </label>
                {isHardcoded(funcConfig?.function) && (
                    <label
                        className="block text-sm font-semibold text-[var(--sea-ink)]"
                        htmlFor={`${id}-hardcoded`}
                    >
                        Function DCC ID:{" "}
                        <input
                            type="number"
                            name={`${id}-hardcoded`}
                            id={`${id}-hardcoded`}
                            value={funcConfig.function.Hardcoded.id}
                            className="my-2 demo-input"
                            min={0}
                            max={31}
                            onChange={(event) => {
                                setFuncConfig((_) => {
                                    const newFuncConfig = structuredClone(
                                        funcConfig,
                                    ) as FunctionConfig & {
                                        function: Hardcoded;
                                    };
                                    newFuncConfig.function.Hardcoded.id =
                                        parseInt(event.target.value, 10);
                                    return newFuncConfig;
                                });
                            }}
                        />
                    </label>
                )}
                {isLabel(funcConfig?.function) && (
                    <label
                        className="block text-sm font-semibold text-[var(--sea-ink)]"
                        htmlFor={`${id}-label`}
                    >
                        Roster Name:{" "}
                        <input
                            type="text"
                            name={`${id}-label`}
                            id={`${id}-label`}
                            value={funcConfig.function.Label.label}
                            className="my-2 demo-input"
                            onChange={(event) => {
                                setFuncConfig((_) => {
                                    const newFuncConfig = structuredClone(
                                        funcConfig,
                                    ) as FunctionConfig & { function: Label };
                                    newFuncConfig.function.Label.label =
                                        event.target.value;
                                    return newFuncConfig;
                                });
                            }}
                        />
                    </label>
                )}
                {(isLabel(funcConfig?.function) ||
                    isHardcoded(funcConfig?.function)) && (
                    <label className="items-center cursor-pointer">
                        <div className="block text-sm font-semibold text-[var(--sea-ink)]">
                            Keep function active while pressed?
                        </div>
                        <div className="py-px-1">
                            <div className="pt-[0.7rem] pb-[0.9rem]">
                                <div className="flex py-2 m-auto justify-center">
                                    <input
                                        type="checkbox"
                                        className="sr-only peer"
                                        checked={
                                            getMomentary(funcConfig.function)
                                                .momentary
                                        }
                                        name={`${id}-momentary`}
                                        onChange={(event) => {
                                            setFuncConfig((_) => {
                                                const newFuncConfig =
                                                    structuredClone(funcConfig);
                                                const value =
                                                    event.target.checked;
                                                getMomentary(
                                                    newFuncConfig.function as
                                                        | Hardcoded
                                                        | Label,
                                                ).momentary = value;
                                                return newFuncConfig;
                                            });
                                        }}
                                    />
                                    <div className="relative w-9 h-5 bg-[var(--sea-ink-soft)] peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-soft dark:peer-focus:ring-brand-soft rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-buffer after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-[var(--palm)]"></div>
                                </div>
                            </div>
                        </div>
                    </label>
                )}

                {funcConfig && (
                    <label
                        htmlFor={`${id}-behavior`}
                        className="block text-sm font-semibold text-[var(--sea-ink)]"
                    >
                        On:
                        <select
                            name={`${id}-behavior`}
                            id={`${id}-behavior`}
                            value={funcConfig.behavior}
                            className="my-2 demo-select"
                            onChange={(event) => {
                                const value = event.target
                                    .value as FunctionBehavior;
                                setFuncConfig((_) => {
                                    const newFuncConfig =
                                        structuredClone(funcConfig);
                                    newFuncConfig.behavior = value;
                                    return newFuncConfig;
                                });
                            }}
                        >
                            {(
                                [
                                    "All",
                                    "Leading",
                                    "Trailing",
                                    "Last",
                                    "Inner",
                                ] as const
                            ).map((name) => (
                                <option value={name} key={name}>
                                    {NICE_BEHAVIOR_LABELS[name]}
                                </option>
                            ))}
                        </select>
                    </label>
                )}
            </div>
        </section>
    );
}

const NICE_BEHAVIOR_LABELS: Record<FunctionBehavior, string> = {
    All: "All units",
    Leading: "Leading unit only",
    Trailing: "Trailing units only",
    Last: "Last unit only",
    Inner: "Inner units only",
};

const NICE_LABELS: Record<FunctionType | "null", string> = {
    Label: "Function by Roster Name",
    Hardcoded: "Function by DCC ID",
    EmergencyStop: "Emergency Stop",
    null: "Nothing",
};
