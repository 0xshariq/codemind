// Original file: ../../proto/v1/common.proto

export const Completeness = {
  COMPLETENESS_UNSPECIFIED: 'COMPLETENESS_UNSPECIFIED',
  COMPLETE: 'COMPLETE',
  INCOMPLETE: 'INCOMPLETE',
  TRUNCATED: 'TRUNCATED',
} as const;

export type Completeness =
  | 'COMPLETENESS_UNSPECIFIED'
  | 0
  | 'COMPLETE'
  | 1
  | 'INCOMPLETE'
  | 2
  | 'TRUNCATED'
  | 3

export type Completeness__Output = typeof Completeness[keyof typeof Completeness]
