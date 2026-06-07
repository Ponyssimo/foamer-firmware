import { createStore } from "@tanstack/react-store";

export const deviceStore = createStore(
    null as {
        usbDevice: USBDevice;
    } | null,
);
