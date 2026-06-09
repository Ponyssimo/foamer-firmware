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
  address: Address;
  functions: [
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
    Function | null,
  ];
};

export type Config = {
  wifi: {
    ssid: string;
    password: string | null;
  };
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

const DEFAULT_PROFILE: Profile = {
  address: {Long: 0x6969},
  functions: new Array(21).fill(null) as any,
};

// TODO: Mary save the config in local storage and read back!!
export const configStore = createStore({
  profiles: new Array(10).fill(DEFAULT_PROFILE),
  wifi_config: {
    ssid: "RIT-WiFi",
    password: null,
  },
} as Config);
