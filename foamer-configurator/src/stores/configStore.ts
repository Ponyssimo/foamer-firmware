import type { UseSelectorOptions } from "@tanstack/react-store";
import { createStore, useSelector } from "@tanstack/react-store";
import { ZodError, z } from "zod";
import { errorStore } from "./errorStore";

// #[derive(Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "defmt", derive(Format))]
// pub enum Function {
//     Label { label: String<32>, momentary: bool },
//     Hardcoded { id: u8, momentary: bool },
//     EmergencyStop,
// }
export type FunctionType = "Label" | "Hardcoded" | "EmergencyStop";

export const USER_BUTTONS: number = 6;
export const TRIPLE_SWITCHES: number = 3;
export const TRIPLE_SWITCH_FUNCTION_COUNT: number = 3;
export const BRAKE_COUNT: number = 5;

export const TRIPLE_SWITCH_START_INDEX: number = USER_BUTTONS;
export const BRAKE_START_INDEX: number =
    TRIPLE_SWITCH_START_INDEX + TRIPLE_SWITCHES * TRIPLE_SWITCH_FUNCTION_COUNT;
export const HORN_INDEX: number = BRAKE_START_INDEX + BRAKE_COUNT;
export const PROFILE_FUNCTION_COUNT: number = HORN_INDEX + 1;
export const MU_COUNT: number = 10;
export const WIFI_COUNT: number = 5;

const DEFAULT_PROFILE: Profile = {
    address: [{ Long: 0x6969 }],
    functions: new Array(21).fill(null),
};

const momentarySchema = {
    momentary: z.boolean(),
};

const hardcodedSchema = z.object({
    Hardcoded: z
        .object({
            id: z.number().max(31),
        })
        .extend(momentarySchema),
});
const labelSchema = z.object({
    Label: z
        .object({
            label: z.string().max(32),
        })
        .extend(momentarySchema),
});
const functionSchema = z.union([
    hardcodedSchema,
    labelSchema,
    z.literal("EmergencyStop"),
]);

const functionBehaviorSchema = z.enum([
    "All",
    "Leading",
    "Trailing",
    "Last",
    "Inner",
]);

const functionConfigSchema = z.object({
    function: functionSchema,
    behavior: functionBehaviorSchema,
});

const addressSchema = z.union([
    z.object({
        Short: z.number().min(0).max(0xff),
    }),
    z.object({
        Long: z.number().min(0).max(0xffff),
    }),
]);

const profileSchema = z.object({
    address: z.array(addressSchema).min(1).max(MU_COUNT),
    functions: z.array(functionConfigSchema.nullable()).length(21),
});

const wifiSchema = z.object({
    ssid: z.string().max(32),
    password: z.string().max(32).nullable(),
});

const wiThrottleDiscoverySchema = z.union([
    z.object({
        Hardcoded: z.string(),
    }),
    z.literal("Mdns"),
]);

const baseConfigSchema = z.object({
    wifi_configs: z.array(wifiSchema).max(WIFI_COUNT),
    withrottle_server: z.object({
        discovery: wiThrottleDiscoverySchema,
    }),
});

export const configSchema = z.object({
    base_config: baseConfigSchema,
    profiles: z.array(profileSchema).length(10),
});

export type Config = z.infer<typeof configSchema>;
export type Function = z.infer<typeof functionSchema>;
export type FunctionBehavior = z.infer<typeof functionBehaviorSchema>;
export type FunctionConfig = z.infer<typeof functionConfigSchema>;
const momentarySchemaWrapped = z.object(momentarySchema);
export type Momentary = z.infer<typeof momentarySchemaWrapped>;
export type WiThrottleDiscovery = z.infer<typeof wiThrottleDiscoverySchema>;
export type WifiConfig = z.infer<typeof wifiSchema>;
export type Profile = z.infer<typeof profileSchema>;
export type BaseConfig = z.infer<typeof baseConfigSchema>;
export type Address = z.infer<typeof addressSchema>;
export type Hardcoded = z.infer<typeof hardcodedSchema>;
export type Label = z.infer<typeof labelSchema>;

// TODO: Mary save the config in local storage and read back!!
export const DEFAULT_CONFIG: Config = {
    profiles: new Array(10)
        .fill(DEFAULT_PROFILE)
        .map((entry) => structuredClone(entry)),
    base_config: {
        withrottle_server: {
            discovery: {
                Hardcoded: "192.0.2.69:12090",
            },
        },
        wifi_configs: [
            {
                ssid: "RIT-WiFi",
                password: null,
            },
        ],
    } as const,
};

export type ConfigStoreValue =
    | {
          type: "Config";
          data: Config;
      }
    | {
          type: "ParsingError";
          message: string | null;
          json: string;
      };

export function configStoreValueToConfig(storeValue: ConfigStoreValue): Config {
    if (storeValue.type == "Config") {
        return storeValue.data;
    }
    throw new Error(
        `Config store has a ParsingError value... This shouldn't be accessible. ${storeValue.message}`,
    );
}

export function useConfig<T = NoInfer<Config>>(
    selector: (snapshot: Config) => T = (s) => s as unknown as T,
    options?: UseSelectorOptions<T>,
): T {
    return useSelector(
        configStore,
        (storeValue: ConfigStoreValue) => {
            const config = configStoreValueToConfig(storeValue);
            return selector(config);
        },
        options,
    );
}

export function setConfig(sentinel: Config | ((config: Config) => Config)) {
    if (typeof sentinel == "function") {
        configStore.setState((storeValue) => {
            const config = configStoreValueToConfig(storeValue);
            return { type: "Config", data: sentinel(config) };
        });
    } else {
        configStore.setState((_) => ({ type: "Config", data: sentinel }));
    }
}

export const configStore = createStore<ConfigStoreValue>({
    type: "Config",
    data: structuredClone(DEFAULT_CONFIG),
});

export function loadConfig(json: string) {
    let initialConfig: Config | undefined;
    try {
        initialConfig = configSchema.parse(JSON.parse(json));
    } catch (err) {
        let message: string | null = null;
        if (typeof err == "string") {
            errorStore.setState((_) => err);
            message = err;
        }
        if (err instanceof ZodError) {
            errorStore.setState((_) =>
                err.issues
                    .map((err) => `.${err.path.join(".")}: ${err.message}`)
                    .join(", "),
            );
        }
        console.error("Failed to parse saved config", err);
        configStore.setState((_) => ({
            type: "ParsingError",
            message,
            json,
        }));
    }
    if (initialConfig) {
        setConfig(initialConfig);
    }
}
