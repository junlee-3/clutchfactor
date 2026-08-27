import { Component, type ErrorInfo, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { EmptyState } from "./EmptyState";

interface Props {
  children: ReactNode;
  resetKey: string;
}
interface State {
  error: Error | null;
}

/** Route-level safety net (spec §2): a render exception shows a calm
 *  fallback instead of a blank window, is logged, and clears when the
 *  route changes (`resetKey`). */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };
  static getDerivedStateFromError(error: Error): State {
    return { error };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("screen crashed", error, info.componentStack);
  }
  componentDidUpdate(prev: Props) {
    if (prev.resetKey !== this.props.resetKey && this.state.error) {
      this.setState({ error: null });
    }
  }
  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="route-crash">
        <EmptyState
          title="Something in this screen broke"
          body={
            <>
              {this.state.error.message} — reload the screen, or{" "}
              <Link to="/">go back to the Library</Link>.
            </>
          }
          action={{ label: "Reload the screen", onClick: () => this.setState({ error: null }) }}
        />
      </div>
    );
  }
}
