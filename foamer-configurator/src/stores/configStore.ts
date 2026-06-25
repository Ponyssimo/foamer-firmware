import {createStore} from "@tanstack/react-store";

// #[derive(Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "defmt", derive(Format))]
// pub enum Function {
//     Label { label: String<32>, momentary: bool },
//     Hardcoded { id: u8, momentary: bool },
//     EmergencyStop,
// }
export type FunctionType = "Label" | "Hardcoded" | "EmergencyStop";
export type Function = Label | Hardcoded | "EmergencyStop";
export type FunctionBehavior =
  | "All"
  | "Leading"
  | "Trailing"
  | "Last"
  | "Inner";
export type FunctionConfig = {
  function: Function;
  behavior: FunctionBehavior;
};

export type Momentary = {
  momentary: boolean;
};

export type Hardcoded = {
  Hardcoded: {
    id: number;
  } & Momentary;
};

export type Label = {
  Label: {
    label: string;
  } & Momentary;
};

export type Address = {Short: number} | {Long: number};

export type Profile = {
  address: Address[];
  functions: [
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
    FunctionConfig | null,
  ];
};

export type WiThrottleDiscovery =
  | {
      Hardcoded: string;
    }
  | "Mdns";

export type BaseConfig = {
  wifi_config: {
    ssid: string;
    password: string | null;
  };
  withrottle_server: {
    discovery: WiThrottleDiscovery;
  };
};

export type Config = {
  base_config: BaseConfig;
  profiles: [
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
  ];
};

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

const DEFAULT_PROFILE: Profile = {
  address: [{Long: 0x6969}],
  functions: new Array(21).fill(null) as any,
};

// TODO: Mary save the config in local storage and read back!!
export const DEFAULT_CONFIG: Config = {
  profiles: new Array(10)
    .fill(DEFAULT_PROFILE)
    .map((entry) => structuredClone(entry)) as [
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
    Profile,
  ],
  base_config: {
    withrottle_server: {
      discovery: {
        Hardcoded: "192.0.2.69:12090",
      },
    },
    wifi_config: {
      ssid: "RIT-WiFi",
      password: null,
    },
  } as const,
};

export const configStore = createStore(structuredClone(DEFAULT_CONFIG));
