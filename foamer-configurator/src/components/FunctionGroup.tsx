import {configStore} from "../stores/configStore";
import {useSelector} from "@tanstack/react-store";

import type {
  Hardcoded,
  Momentary,
  Label,
  Function,
  FunctionType,
} from "../stores/configStore";

export function FunctionGroup({
  groupName,
  start,
  profileId,
  functions,
  typeLabel,
}: {
  profileId: number;
  groupName: string;
  start: number;
  functions: string[];
  typeLabel: (string, number) => string;
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

const DEFAULT_FUNCTION_BY_TYPE: {[K in FunctionType]: Function} = {
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

function getFunctionType(func: Function | null): FunctionType | null {
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

function isHardcoded(func: Function | null): func is Hardcoded {
  return getFunctionType(func) == "Hardcoded";
}

function isLabel(func: Function | null): func is Label {
  return getFunctionType(func) == "Label";
}

function isEmergencyStop(func: Function | null): func is "EmergencyStop" {
  return getFunctionType(func) == "EmergencyStop";
}
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
  const func = useSelector(
    configStore,
    (config) => config.profiles[profileId].functions[index],
  );

  const setFunc = (sentinel: (func: Function | null) => Function | null) => {
    configStore.setState((config) => {
      config = structuredClone(config);
      const oldFunc = config.profiles[profileId].functions[index];
      const newFunc = sentinel(oldFunc);
      config.profiles[profileId].functions[index] = newFunc;
      return config;
    });
  };

  // console.log("Type", getFunctionType(func));
  const id = `function-${index}`;
  return (
    <section className="island-shell mt-8 rounded-2xl p-6">
      <p className="island-kicker mb-2">{name}</p>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <label
          htmlFor={id}
          className="block text-sm font-semibold text-[var(--sea-ink)]"
        >
          {typeLabel}, activate:{" "}
          <select
            name={id}
            value={getFunctionType(func) ?? "null"}
            className="my-2 demo-select"
            onChange={(event) => {
              const value = event.target.value as FunctionType | "null";
              const defaultType =
                value == "null" ? null : DEFAULT_FUNCTION_BY_TYPE[value];
              setFunc((_) => structuredClone(defaultType));
            }}
          >
            {(["null", "Label", "Hardcoded", "EmergencyStop"] as const).map(
              (name) => (
                <option value={name} key={name}>
                  {NICE_LABELS[name]}
                </option>
              ),
            )}
          </select>
        </label>
        {isHardcoded(func) && (
          <label
            className="block text-sm font-semibold text-[var(--sea-ink)]"
            htmlFor={`${id}-hardcoded`}
          >
            Function DCC ID:{" "}
            <input
              type="number"
              name={`${id}-hardcoded`}
              value={func.Hardcoded.id}
              className="my-2 demo-input"
              min={0}
              max={31}
              onChange={(event) => {
                setFunc((_) => {
                  const newFunc = structuredClone(func);
                  newFunc.Hardcoded.id = parseInt(event.target.value);
                  return newFunc;
                });
              }}
            />
          </label>
        )}
        {isLabel(func) && (
          <label
            className="block text-sm font-semibold text-[var(--sea-ink)]"
            htmlFor={`${id}-label`}
          >
            Roster Name:{" "}
            <input
              type="text"
              name={`${id}-label`}
              value={func.Label.label}
              className="my-2 demo-input"
              onChange={(event) => {
                setFunc((_) => {
                  const newFunc = structuredClone(func);
                  newFunc.Label.label = event.target.value;
                  return newFunc;
                });
              }}
            />
          </label>
        )}
        {(isLabel(func) || isHardcoded(func)) && (
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
                    checked={getMomentary(func).momentary}
                    name={`${id}-momentary`}
                    onChange={(event) => {
                      setFunc((_) => {
                        const newFunc = structuredClone(func);
                        const value = event.target.checked;
                        getMomentary(newFunc).momentary = value;
                        return newFunc;
                      });
                    }}
                  />
                  <div className="relative w-9 h-5 bg-[var(--sea-ink-soft)] peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-brand-soft dark:peer-focus:ring-brand-soft rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-buffer after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-[var(--palm)]"></div>
                </div>
              </div>
            </div>
          </label>
        )}
      </div>
    </section>
  );
}

const NICE_LABELS: {[K in FunctionType | "null"]: string} = {
  Label: "Function by Roster Name",
  Hardcoded: "Function by DCC ID",
  EmergencyStop: "Emergency Stop",
  null: "Nothing",
};
