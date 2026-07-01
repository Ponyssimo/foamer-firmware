import { AddressSelector } from "../components/AddressSelector";
import { MU_COUNT, setConfig, useConfig } from "../stores/configStore";

export function Consist({ profileId }: { profileId: number }) {
    const addresses = useConfig((config) => config.profiles[profileId].address);

    return (
        <div>
            {addresses.map((_, addressId) => (
                <div
                    key={addressId}
                    className="flex flex-col md:flex-row md:gap-4"
                >
                    <div className="flex-1">
                        <AddressSelector
                            value={addresses[addressId]}
                            onChange={(value) =>
                                setConfig((config) => {
                                    config = structuredClone(config);
                                    config.profiles[profileId].address[
                                        addressId
                                    ] = value;
                                    return config;
                                })
                            }
                        />
                    </div>
                    <div className="flex justify-end flex-col">
                        <label>
                            <button
                                type="button"
                                className="demo-button-input text-sm my-2"
                                onClick={() => {
                                    setConfig((config) => {
                                        config = structuredClone(config);
                                        config.profiles[
                                            profileId
                                        ].address.splice(addressId, 1);
                                        return config;
                                    });
                                }}
                                disabled={addresses.length <= 1}
                            >
                                Remove
                            </button>
                        </label>
                    </div>
                </div>
            ))}
            <button
                type="button"
                className="demo-button"
                onClick={() => {
                    setConfig((config) => {
                        config = structuredClone(config);
                        config.profiles[profileId].address.push({
                            Long: 0x6969,
                        });
                        return config;
                    });
                }}
                disabled={addresses.length >= MU_COUNT}
            >
                Add Unit
            </button>
        </div>
    );
}
