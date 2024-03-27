import type { Status } from "@/components/CaseIndicator.astro";

export type WebSocketRequest =
    | {
          type: "judge";
          program: string;
          language: string;
      }
    | {
          type: "test";
          program: string;
          language: string;
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

export default (
    contestId: string,
    problemId: string,
    runMessageWrapper: HTMLElement,
    runMessage: HTMLElement,
    debugCaseIndicator: HTMLElement,
    testOutput: HTMLTextAreaElement,
    toggleButtons: (disabled: boolean) => void
) => {
    const url = `ws://${window.location.host}/run/ws/${contestId}/${problemId}`;
    console.debug("Connecting to WebSocket at", url);
    const ws = new WebSocket(url);

    const stateIsComplete = (state: JobState) => {
        switch (state.type) {
            case "judging":
                return !state.cases.some((c) => c.status === "pending" || c.status === "running");
            case "testing":
                return state.status.status !== "pending" && state.status.status !== "running";
        }
    };

    const typeToStatus: Record<CaseStatus["status"], Status> = {
        failed: "error",
        passed: "success",
        notRun: "empty",
        pending: "idle",
        running: "loading"
    };

    ws.onopen = () => {
        console.debug("WebSocket connection established");
        toggleButtons(false);
    };

    ws.onmessage = (event) => {
        const message: WebSocketMessage = JSON.parse(event.data);
        console.debug("Received message", message);

        switch (message.type) {
            case "stateUpdate":
                const state = message.state as JobState;
                const complete = stateIsComplete(state);
                toggleButtons(!complete);
                switch (state.type) {
                    case "judging":
                        for (const [i, c] of state.cases.entries()) {
                            document
                                .querySelector(`[data-case-number='${i}']`)!
                                .setAttribute("data-status", typeToStatus[c.status]);
                        }
                        if (complete) {
                            const firstWithErr = state.cases.find((c) => c.status === "failed");
                            if (firstWithErr && firstWithErr.status === "failed") {
                                runMessageWrapper.setAttribute("data-status", "error");
                                runMessage.innerText = firstWithErr.content;
                            } else {
                                runMessageWrapper.setAttribute("data-status", "success");
                                runMessage.innerText = "Passed!";
                            }
                        } else {
                            runMessageWrapper.setAttribute("data-status", "loading");
                            runMessage.innerText = "Running...";
                        }
                        break;
                    case "testing":
                        debugCaseIndicator.setAttribute(
                            "data-status",
                            typeToStatus[state.status.status]
                        );
                        switch (state.status.status) {
                            case "passed":
                                testOutput.value = state.status.content ?? "";
                                break;
                            case "failed":
                                testOutput.value = state.status.content ?? "";
                                break;
                        }
                }
                break;
            case "invalid":
                console.error("Invalid message sent", message);
                break;
            case "runDenied":
                runMessageWrapper.setAttribute("data-status", "error");
                runMessage.innerText = message.reason;
                break;
            case "runStarted":
                toggleButtons(true);
                break;
        }
    };

    ws.onclose = () => {
        console.debug("WebSocket connection closed");
        toggleButtons(true);
        runMessageWrapper.setAttribute("data-status", "disconnected");
        runMessage.innerText = "Disconnected, please refresh the page.";
    };

    ws.onerror = (error) => {
        console.error("WebSocket error:", error);
    };

    return ws;
};
