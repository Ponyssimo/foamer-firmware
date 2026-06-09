import {useState} from "react";
import {createFileRoute} from "@tanstack/react-router";
import {
  configStore,
  BRAKE_START_INDEX,
  TRIPLE_SWITCHES,
  TRIPLE_SWITCH_FUNCTION_COUNT,
  TRIPLE_SWITCH_START_INDEX,
} from "../stores/configStore";
import {useSelector} from "@tanstack/react-store";
import {AddressSelector} from "../components/AddressSelector";
import {FunctionGroup} from "../components/FunctionGroup";
//import {greet} from "../../pkg";

export const Route = createFileRoute("/")({component: App});

function App() {
  const [profileId, setProfileId] = useState(0);
  const profile = useSelector(
    configStore,
    (config) => config.profiles[Number(profileId)],
  );

  return (
    <main className="page-wrap px-4 pb-8 pt-14">
      <section className="island-shell mt-8 rounded-2xl p-6">
        <p className="island-kicker mb-2">Profiles</p>
        <label
          htmlFor="profile"
          className="block text-sm font-semibold text-[var(--sea-ink)]"
        >
          Profile
          <select
            name="profile"
            value={profileId}
            className="my-2 demo-select"
            onChange={(event) => {
              setProfileId(Number(event.target.value));
            }}
          >
            {new Array(10).fill(null).map((_, key) => (
              <option value={key} key={key}>
                {key}
              </option>
            ))}
          </select>
        </label>
        <AddressSelector
          value={profile.address}
          onChange={(value) =>
            configStore.setState((config) => {
              config = structuredClone(config);
              config.profiles[profileId].address = value;
              return config;
            })
          }
        />

        <FunctionGroup
          groupName="Function Buttons"
          typeLabel={(name) => `When ${name} is pressed`}
          start={0}
          profileId={profileId}
          functions={new Array(4)
            .fill(null)
            .map((_, index) => `User ${index + 1}`)
            .concat(["Bell", "Dynamics"])}
        />

        <FunctionGroup
          groupName="Lights"
          typeLabel={(_name, index) =>
            `When ${TRIPLE_SWITCH_LABELS[Math.floor(index / TRIPLE_SWITCH_FUNCTION_COUNT)]} switch is in the ${TRIPLE_SWITCH_POSITION_LABELS[index % TRIPLE_SWITCH_FUNCTION_COUNT]} position`
          }
          start={TRIPLE_SWITCH_START_INDEX}
          profileId={profileId}
          functions={new Array(TRIPLE_SWITCHES * TRIPLE_SWITCH_FUNCTION_COUNT)
            .fill(null)
            .map((_, index) => {
              const positionId = index % TRIPLE_SWITCH_FUNCTION_COUNT;
              const switchId = Math.floor(index / TRIPLE_SWITCH_FUNCTION_COUNT);
              return `${TRIPLE_SWITCH_LABELS[switchId]} ${TRIPLE_SWITCH_POSITION_LABELS[positionId]}`;
            })}
        />

        <FunctionGroup
          groupName="Brake"
          typeLabel={(name) => `When brake handle is in ${name}`}
          start={BRAKE_START_INDEX}
          profileId={profileId}
          functions={BRAKE_LABELS}
        />

        <ul className="m-0 list-disc space-y-2 pl-5 text-sm text-[var(--sea-ink-soft)]">
          <li>
            Edit <code>src/routes/index.tsx</code> to customize the home page.
          </li>
          <li>
            Update <code>src/components/Header.tsx</code> and{" "}
            <code>src/components/Footer.tsx</code> for brand links.
          </li>
          <li>
            Add routes in <code>src/routes</code> and tweak visual tokens in{" "}
            <code>src/styles.css</code>.
          </li>
        </ul>
      </section>
    </main>
  );
}

const TRIPLE_SWITCH_LABELS: [string, string, string] = [
  "Ditch Lights",
  "Rear Headlights",
  "Front Headlights",
];

const TRIPLE_SWITCH_POSITION_LABELS: [string, string, string] = [
  "Up",
  "Middle",
  "Down",
];

const BRAKE_LABELS: [string, string, string, string, string] = [
  "Brake Released Position",
  "Brake Step 1",
  "Brake Step 2",
  "Brake Step 3",
  "Emergency Brake Position",
];
