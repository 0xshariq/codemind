// Original file: ../../proto/v1/common.proto

export const Freshness = {
  FRESHNESS_UNSPECIFIED: 'FRESHNESS_UNSPECIFIED',
  CURRENT: 'CURRENT',
  STALE: 'STALE',
} as const;

export type Freshness =
  | 'FRESHNESS_UNSPECIFIED'
  | 0
  | 'CURRENT'
  | 1
  | 'STALE'
  | 2

export type Freshness__Output = typeof Freshness[keyof typeof Freshness]
