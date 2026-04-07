import type { IconType } from "react-icons";
import {
  MdAccountTree,
  MdDesktopWindows,
  MdExpandLess,
  MdExpandMore,
  MdFolder,
  MdGroupWork,
  MdHistory,
  MdIntegrationInstructions,
  MdLaptopMac,
  MdSettingsEthernet,
  MdSmartToy,
  MdTerminal,
  MdTune,
  MdAutoFixHigh,
} from "react-icons/md";
import { FaApple, FaWindows, FaLinux } from "react-icons/fa";

const iconMap: Record<string, IconType> = {
  account_tree: MdAccountTree,
  auto_fix_high: MdAutoFixHigh,
  desktop_windows: MdDesktopWindows,
  expand_less: MdExpandLess,
  expand_more: MdExpandMore,
  folder: MdFolder,
  group_work: MdGroupWork,
  history: MdHistory,
  integration_instructions: MdIntegrationInstructions,
  laptop_mac: MdLaptopMac,
  robot_2: MdSmartToy,
  settings_ethernet: MdSettingsEthernet,
  smart_toy: MdSmartToy,
  terminal: MdTerminal,
  tune: MdTune,
  apple: FaApple,
  windows: FaWindows,
  linux: FaLinux,
};

export function LandingIcon({
  name,
  className = "",
}: {
  name: string;
  className?: string;
}) {
  const Icon = iconMap[name];

  if (!Icon) {
    return (
      <span className={className} aria-hidden="true">
        {name}
      </span>
    );
  }

  return <Icon className={className} aria-hidden="true" />;
}
