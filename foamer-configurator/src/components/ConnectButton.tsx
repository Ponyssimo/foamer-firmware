import { useSelector } from "@tanstack/react-store";
import { useEffect } from "react";
import type { Config } from "../stores/configStore";
import { configStore } from "../stores/configStore";
import { deviceStore } from "../stores/deviceStore";
import { errorStore } from "../stores/errorStore";
import { wasmPromise } from "../wasm";

const INTERFACE_NUMBER = 1;
const ENDPOINT_NUMBER = 1;

function getPacketSize(device: USBDevice): number {
    const inEndpoint = device
        .configuration!.interfaces.find(
            (iface) => iface.interfaceNumber == INTERFACE_NUMBER,
        )!
        .alternate.endpoints.find(
            (endpoint) =>
                endpoint.endpointNumber == ENDPOINT_NUMBER &&
                endpoint.direction == "in",
        )!;
    return inEndpoint.packetSize;
}

async function loadConfig(device: USBDevice): Promise<Config> {
    const wasm = await wasmPromise;

    const packetSize = getPacketSize(device);
    device.transferOut(1, wasm.create_read_request().slice());
    const transfer = await device.transferIn(ENDPOINT_NUMBER, packetSize);
    const data = new Uint8Array(transfer.data!.buffer);
    console.log("Data", data);
    const response = JSON.parse(wasm.decode_in_control_message(data));
    console.log("Response", response);

    const configLength = response.ReadConfig.length;
    const configBuffer = new Uint8Array(configLength);
    for (let offset = 0; offset < configLength; offset += packetSize) {
        console.log("Doing another in transfer for", offset);
        const transfer = await device.transferIn(ENDPOINT_NUMBER, packetSize);
        const data = new Uint8Array(transfer.data!.buffer);
        configBuffer.set(
            data.slice(0, Math.min(data.length, configBuffer.length - offset)),
            offset,
        );
    }

    console.log("Got config buffer", configBuffer);
    const config = JSON.parse(wasm.decode_config(configBuffer));
    console.log("Config", config);

    return config;
}

async function saveConfig(device: USBDevice, config: Config) {
    const wasm = await wasmPromise;

    const packetSize = getPacketSize(device);
    let configBuffer: Uint8Array;
    try {
        configBuffer = wasm.encode_config(JSON.stringify(config));
    } catch (err) {
        if (typeof err != "string") {
            throw err;
        }
        errorStore.setState((_) => err);
        return;
    }
    const configTmp = wasm.decode_config(configBuffer);
    console.log("Got config buffer", configBuffer, configTmp);
    // const textEncoder = new TextEncoder();
    // const configBuffer = textEncoder.encode(JSON.stringify(config));
    await device.transferOut(
        ENDPOINT_NUMBER,
        wasm.create_write_request(configBuffer.length).slice(),
    );

    for (let offset = 0; offset < configBuffer.length; offset += packetSize) {
        await device.transferOut(
            ENDPOINT_NUMBER,
            configBuffer.slice(offset, offset + packetSize),
        );
    }

    console.log("Theoretically we sent the config...");
}

export default function ConnectButton() {
    const error = useSelector(errorStore, (state) => state);
    const device = useSelector(deviceStore, (state) => state?.usbDevice);

    useEffect(() => {
        let destroyed = false;
        navigator.usb.getDevices().then(async (devices) => {
            console.log("Got devices", devices);
            if (!destroyed && devices.length > 0) {
                await claimDevice(devices[0]);
            }
        });

        return () => {
            destroyed = true;
        };
    }, []);

    useEffect(() => {
        const connectCallback = (event: USBConnectionEvent) => {
            if (!deviceStore.state?.usbDevice) {
                claimDevice(event.device);
            }
        };
        const disconnectCallback = (event: USBConnectionEvent) => {
            if (event.device) {
                deviceStore.setState((_) => null);
            }
        };
        navigator.usb.addEventListener("connect", connectCallback);
        navigator.usb.addEventListener("disconnect", disconnectCallback);
        return () => {
            navigator.usb.removeEventListener("connect", connectCallback);
            navigator.usb.removeEventListener("disconnect", connectCallback);
        };
    }, []);

    return (
        <details className="rounded-full border border-[var(--chip-line)] bg-[var(--chip-bg)] px-3 py-1.5 text-sm font-semibold text-[var(--sea-ink)] shadow-[0_8px_22px_rgba(30,90,72,0.08)] transition hover:-translate-y-0.5">
            <summary>
                {device
                    ? `Device: ${device.productName} - ${device.serialNumber}`
                    : "Connect Device"}
            </summary>
            <div className="mt-2 min-w-56 rounded-xl border border-[var(--line)] bg-[var(--header-bg)] p-2 shadow-lg sm:absolute sm:right-0">
                {device ? (
                    <>
                        <button
                            className="block cursor-pointer"
                            type="button"
                            onClick={() => {
                                device
                                    .close()
                                    .catch((err) => {
                                        console.warn(
                                            "Failed to disconnect",
                                            err,
                                        );
                                    })
                                    .then(() => {
                                        deviceStore.setState((_state) => null);
                                    });
                            }}
                        >
                            Disconnect {device?.productName}
                        </button>
                        <button
                            className="block cursor-pointer"
                            type="button"
                            onClick={() => {
                                console.log("Loading config from", device);
                                loadConfig(device).then((config) => {
                                    console.log("Got config", config);
                                    configStore.setState((_state) => config);
                                });
                            }}
                        >
                            Load config from device
                        </button>
                        <button
                            className={`block${error ? " text-red-600" : ""}`}
                            type="button"
                            disabled={!!error}
                            onClick={() => {
                                saveConfig(device, configStore.get());
                            }}
                        >
                            {error
                                ? "Bad config, save unavailable"
                                : "Save config to device"}
                        </button>
                    </>
                ) : (
                    <button
                        className="block cursor-pointer"
                        type="button"
                        onClick={() => {
                            navigator.usb
                                .requestDevice({
                                    filters: [
                                        { vendorId: 0x0403, productId: 0x698f },
                                    ],
                                })
                                .then(async (usbDevice) => {
                                    await claimDevice(usbDevice);
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

async function claimDevice(usbDevice: USBDevice) {
    console.log("Got device", usbDevice);
    await usbDevice.open();
    console.log("opened device", usbDevice);
    await usbDevice.selectConfiguration(1);
    console.log("select config 1");
    await usbDevice.claimInterface(1);
    console.log("claim if 1");
    deviceStore.setState((_state) => ({ usbDevice }));
}
