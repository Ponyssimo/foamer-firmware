import {useSelector} from "@tanstack/react-store";
import {errorStore} from "../stores/errorStore";

export function ErrorBanner() {
  const error = useSelector(errorStore, (error) => error);
  if (!error) {
    return null;
  }
  return (
    <div className="bg-red-200 px-6 py-4 mx-2 my-4 rounded-md text-lg flex items-center mx-auto max-w-lg">
      <span className="text-red-800"> {error} </span>
    </div>
  );
}
