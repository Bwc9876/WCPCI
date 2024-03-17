export type WebSocketRequest =
    | {
          type: "judge";
          program: string;
      }
    | {
          type: "test";
          program: string;
          input: string;
      };

export type CaseStatus =
    | {
          status: "running";
      }
    | {
          status: "pending";
      }
    | {
          status: "passed";
          content: string | null;
      }
    | {
          status: "failed";
          content: string;
      }
    | {
          status: "notRun";
      };

export type JobState =
    | {
          type: "judging";
          cases: CaseStatus[];
      }
    | {
          type: "testing";
          status: CaseStatus;
      };

export type WebSocketMessage =
    | {
          type: "stateUpdate";
          state: JobState;
      }
    | {
          type: "runStarted";
      }
    | {
          type: "runDenied";
          reason: string;
      }
    | {
          type: "invalid";
          error: string;
      };
