import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.cm as cm
import matplotlib.colors as mcolors


def plot(data, include_times,zoom, output_filename):
    df = pd.DataFrame(data)
    df = df.sort_values(by=["fut_name", "start"], ascending=True)

    unique_thread_ids = df["thread_id"].drop_duplicates().tolist()

    thread_ids_len = len(unique_thread_ids)

    cmap = cm.get_cmap("tab20", thread_ids_len)
    thread_id_color_map = {tid: cmap(i) for i, tid in enumerate(unique_thread_ids)}
    
    unique_names = df["fut_name"].unique()

    plt.figure(figsize=(10, len(unique_names)))

    scale_factor = 2
    margin = 0.1
    padding = 0.1

    max_end = df["end"].max()
    zoom_threahold = max_end * zoom

    df = df[df["end"] <= zoom_threahold]

    for i, fut_name in enumerate(unique_names):
        subset = df[df["fut_name"] == fut_name]

        # Offset the actual 'value' by i * scale_factor
        y_vals = subset["value"] + i * (scale_factor + padding * 2 + margin * 2)

        plt.scatter(
            (subset["start"] + subset["end"]) / 2, y_vals, s=2, color="#ddd", zorder=2
        )

        if include_times:
            for row in subset.itertuples():
                plt.broken_barh(
                    [(row.start, row.end - row.start)],
                    (
                        i * (scale_factor + padding * 2 + margin * 2)
                        - scale_factor / 2
                        - padding / 2,
                        scale_factor + padding,
                    ),  
                    facecolors=thread_id_color_map[row.thread_id],
                    edgecolors="none",
                    zorder=1,
                    alpha=0.6,
                )

    plt.style.use(
        "https://github.com/dhaitz/matplotlib-stylesheets/raw/master/pitayasmoothie-dark.mplstyle"
    )

    # Label the ticks using fut names
    plt.yticks(
        [
            i * (scale_factor + padding * 2 + margin * 2)
            for i in range(len(unique_names))
        ],
        unique_names,
    )
    plt.gca().invert_yaxis()
    plt.gca().xaxis.set_label_text("Time (microseconds)")
    plt.gca().xaxis.label.set_fontstyle("italic")
    
    plt.savefig(output_filename, bbox_inches="tight")
