import sys

from downloaders import ALL_CHAINS

if __name__ == "__main__":
    # chain name -> (success_count, failure_count)
    summary = {}

    for ChainClass in ALL_CHAINS:
        chain = ChainClass()
        name = ChainClass.__name__
        try:
            chain.download()
            summary[name] = (chain.success_count, chain.failure_count)
        except Exception as e:  # one chain must never abort the whole run
            print(f"ERROR: {name} raised: {e}")
            summary[name] = (0, 0)

    # Per-chain summary — a chain silently going to zero is the most common
    # failure mode (retailers change their sites without notice), so make it
    # visible in every pipeline log (ARCHITECTURE.md §4.2).
    print("\n===== Pipeline summary =====")
    empty_chains = []
    for name, (ok, failed) in summary.items():
        print(f"  {name:<12} {ok:>4} downloaded, {failed:>4} failed")
        if ok == 0:
            empty_chains.append(name)

    if empty_chains:
        print(f"WARNING: chains with 0 downloaded files: {', '.join(empty_chains)}")

    # Exit non-zero only when *nothing* was downloaded at all — that means the
    # whole pipeline is broken (network down / all listings changed), which
    # run_pipeline.sh logs as an error.
    if summary and all(ok == 0 for ok, _ in summary.values()):
        print("ERROR: no chain downloaded any file.")
        sys.exit(1)
