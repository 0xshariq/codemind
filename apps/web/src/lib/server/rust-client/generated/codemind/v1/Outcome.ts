// Original file: /home/sharique/desktop/hackathon/codemind/proto/v1/common.proto

export const Outcome = {
  OUTCOME_UNSPECIFIED: 'OUTCOME_UNSPECIFIED',
  SUCCESS: 'SUCCESS',
  EMPTY: 'EMPTY',
  PARTIAL: 'PARTIAL',
  UNSUPPORTED: 'UNSUPPORTED',
  INVALID: 'INVALID',
  UNAVAILABLE: 'UNAVAILABLE',
  FAILED: 'FAILED',
} as const;

export type Outcome =
  | 'OUTCOME_UNSPECIFIED'
  | 0
  | 'SUCCESS'
  | 1
  | 'EMPTY'
  | 2
  | 'PARTIAL'
  | 3
  | 'UNSUPPORTED'
  | 4
  | 'INVALID'
  | 5
  | 'UNAVAILABLE'
  | 6
  | 'FAILED'
  | 7

export type Outcome__Output = typeof Outcome[keyof typeof Outcome]
