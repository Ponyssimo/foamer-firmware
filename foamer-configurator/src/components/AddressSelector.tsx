import type {Address} from "../stores/configStore";

export function AddressSelector({
  value,
  onChange,
}: {
  value: Address;
  onChange: (address: Address) => unknown;
}) {
  const type = "Long" in value ? "Long" : "Short";

  function onChangeWrapped(address: Address) {
    console.log("On change wrapped", address);
    if ("Long" in address) {
      address.Long &= 0xffff;
    } else if ("Short" in address) {
      address.Short &= 0xff;
    }
    onChange(address);
  }

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      <label
        htmlFor="address"
        className="block text-sm font-semibold text-[var(--sea-ink)]"
      >
        Locomotive Address Type
        <select
          name="addressType"
          className="my-2 demo-select"
          value={type}
          onChange={(event) => {
            onChangeWrapped({
              [event.target.value]: value[type],
            });
          }}
        >
          <option value="Long">Long</option>
          <option value="Short">Short</option>
        </select>
      </label>
      <label
        htmlFor="address-number"
        className="block text-sm font-semibold text-[var(--sea-ink)]"
      >
        Address
        <input
          type="text"
          name="address-number"
          className="my-2 demo-input"
          value={value[type].toString(16)}
          maxLength={type == "Long" ? 4 : 2}
          onChange={(event) => {
            onChangeWrapped({
              [type]: parseInt(event.target.value, 16),
            });
          }}
        />
      </label>
    </div>
  );
}
