import { useState } from "react";

interface LocationFlagProps {
  location: string;
}

export function LocationFlag({ location }: LocationFlagProps) {
  const [failed, setFailed] = useState(false);
  const code = location.toLowerCase();

  if (failed || !/^[a-z]{2}$/.test(code)) {
    return <>{location}</>;
  }

  return (
    <div className="flex justify-center">
      <img
        className="h-5 w-5"
        src={`static/flags/4x3/${code}.svg`}
        alt={location}
        onError={() => setFailed(true)}
      />
    </div>
  );
}
