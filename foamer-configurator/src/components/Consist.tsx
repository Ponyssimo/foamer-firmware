import {useSelector} from "@tanstack/react-store";
import {MU_COUNT, configStore} from "../stores/configStore";
import {AddressSelector} from "../components/AddressSelector";

export function Consist({profileId}: {profileId: number}) {
  const addresses = useSelector(
    configStore,
    (config) => config.profiles[profileId].address,
  );

  return (
    <div>
      {addresses.map((_, addressId) => (
        <div key={addressId}>
          <AddressSelector
            value={addresses[addressId]}
            onChange={(value) =>
              configStore.setState((config) => {
                config = structuredClone(config);
                config.profiles[profileId].address[addressId] = value;
                return config;
              })
            }
          />
          <button
            type="button"
            className="demo-button"
            onClick={() => {
              configStore.setState((config) => {
                config = structuredClone(config);
                config.profiles[profileId].address.splice(addressId, 1);
                return config;
              });
            }}
            disabled={addressId == 0}
          >
            Remove
          </button>
        </div>
      ))}
      {addresses.length < MU_COUNT && (
        <button
          type="button"
          className="demo-button"
          onClick={() => {
            configStore.setState((config) => {
              config = structuredClone(config);
              config.profiles[profileId].address.push({Long: 0x6969});
              return config;
            });
          }}
        >
          Add
        </button>
      )}
    </div>
  );
}
