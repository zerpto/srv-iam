import type { Address, Region } from "@zerpto/sdk";

/** Address fields in region depth order: depth 1 is the country, depth 5 the village. */
export const ADDRESS_LEVELS = [
  "country",
  "province",
  "district",
  "subDistrict",
  "village",
] as const;
export type AddressLevel = (typeof ADDRESS_LEVELS)[number];

/** The parts of a `Region` the form keeps once it has been picked. */
export type PickedRegion = Pick<
  Region,
  "id" | "depth" | "levelLabel" | "code" | "name" | "postalCode" | "hasChildren" | "requiredDepth"
>;

export type AddressValue = {
  /** Deepest region picked ("" when the address is typed by hand). */
  regionId: string;
  country: string;
  province: string;
  district: string;
  subDistrict: string;
  village: string;
  postalCode: string;
  line1: string;
  line2: string;
  /** Regions picked so far, country first. */
  picked: PickedRegion[];
  /** The next level the user still has to pick, when the country requires more depth. */
  incompleteLevel: AddressLevel | "";
};

export const emptyAddress: AddressValue = {
  regionId: "",
  country: "",
  province: "",
  district: "",
  subDistrict: "",
  village: "",
  postalCode: "",
  line1: "",
  line2: "",
  picked: [],
  incompleteLevel: "",
};

export function pickedFrom(region: Region): PickedRegion {
  const { id, depth, levelLabel, code, name, postalCode, hasChildren, requiredDepth } = region;
  return { id, depth, levelLabel, code, name, postalCode, hasChildren, requiredDepth };
}

/** The chain country → … → `region` from a region fetched with `include=parent.parent…`. */
export function regionChain(region: Region): PickedRegion[] {
  const chain: PickedRegion[] = [];
  for (let current: Region | null | undefined = region; current; current = current.parent) {
    chain.unshift(pickedFrom(current));
  }
  return chain;
}

/** True once the user has typed or picked anything, so the address must then be valid. */
export function isAddressStarted(a: AddressValue) {
  return Boolean(
    a.country ||
    a.regionId ||
    a.province ||
    a.district ||
    a.subDistrict ||
    a.village ||
    a.postalCode ||
    a.line1 ||
    a.line2,
  );
}

export function addressFromApi(address: Address | null | undefined): AddressValue {
  if (!address) return emptyAddress;
  return {
    regionId: address.regionId ?? "",
    country: address.country ?? "",
    province: address.province ?? "",
    district: address.district ?? "",
    subDistrict: address.subDistrict ?? "",
    village: address.village ?? "",
    postalCode: address.postalCode ?? "",
    line1: address.line1 ?? "",
    line2: address.line2 ?? "",
    picked: [], // Restored from `regionId` by <AddressFields>.
    incompleteLevel: "",
  };
}

/** The request address, or `undefined` when the section was left empty. Empty fields are left out. */
export function addressToBody(a: AddressValue): Address | undefined {
  if (!isAddressStarted(a)) return undefined;
  const text = (value: string) => value.trim() || undefined;
  return {
    regionId: a.regionId || undefined,
    country: text(a.country)?.toUpperCase(),
    province: text(a.province),
    district: text(a.district),
    subDistrict: text(a.subDistrict),
    village: text(a.village),
    postalCode: text(a.postalCode),
    line1: text(a.line1) ?? "",
    line2: text(a.line2),
  };
}

/** A saved address as display lines (street, area, province and postcode, country); empty parts are left out. */
export function addressLines(address: Address | null | undefined): string[] {
  return [
    [address?.line1, address?.line2].filter(Boolean).join(", "),
    [address?.village, address?.subDistrict, address?.district].filter(Boolean).join(", "),
    [address?.province, address?.postalCode].filter(Boolean).join(" "),
    address?.country ?? "",
  ].filter(Boolean);
}
