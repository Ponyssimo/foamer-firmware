import { useEffect, useState } from "react";
import { useSelector } from "@tanstack/react-store";
import { deviceStore } from "../stores/deviceStore";

type Profile = {
  address: {short: number} | {long: number};
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
  ]
};

type Config = {
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

async function loadConfig(device: USBDevice): Config {
  const wasm = await import("../../pkg");
  device.transferOut(1, wasm.create_read_request());
  const data = await device.transferIn(1, 64);
  console.log("Data", data);
  const response = wasm.decode_in_control_message(data);
  console.log("Response", response);
}

export default function ConnectButton() {
    const device = useSelector(deviceStore, (state) => state?.usbDevice);
    const label = device
        ? `Connected to ${device.productName}`
        : "Disconnected. Click to connect";

    return (
        <details className="rounded-full border border-[var(--chip-line)] bg-[var(--chip-bg)] px-3 py-1.5 text-sm font-semibold text-[var(--sea-ink)] shadow-[0_8px_22px_rgba(30,90,72,0.08)] transition hover:-translate-y-0.5">
            <summary>
                {device ? `Device: ${device.productName}` : "Connect Device"}
            </summary>
            <div className="mt-2 min-w-56 rounded-xl border border-[var(--line)] bg-[var(--header-bg)] p-2 shadow-lg sm:absolute sm:right-0">
                {device ? (
                  <>
                    <button
                        type="button"
                        onClick={() => {
                            device.close().then(() => {
                                deviceStore.setState((_state) => null);
                            });
                        }}
                    >
                        Disconnect {device?.productName}
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        console.log("Loading config from", device);
                        loadConfig(device).then(config => {
                          console.log("Got config", config);
  configStore.setState((_state) => config);
                        });
                      }}
                    >
                      Load config from device
                    </button>
                    <button
                      type="button"
                      onClick={() => {

                      }}
                    >
                      Save config to device
                    </button>
                  </>
                ) : (
                    <button
                        type="button"
                        className="cursor-pointer"
                        onClick={() => {
                            navigator.usb
                                .requestDevice({
                                    filters: [
                                        { vendorId: 0x1209, productId: 0x0001 },
                                    ],
                                })
                                .then(async (usbDevice) => {
                              console.log("Got device", usbDevice);
                                    await usbDevice.open();
                              console.log("opened device", usbDevice);
                                    await usbDevice.selectConfiguration(1);
                              console.log("select config 1");
                                    await usbDevice.claimInterface(1);
                              console.log("claim if 1");
                              deviceStore.setState((_state) => ({usbDevice}));
                                });
                        }}
                    >
                        Connect
                    </button>
                )}
            </div>
        </details>
    );
}
