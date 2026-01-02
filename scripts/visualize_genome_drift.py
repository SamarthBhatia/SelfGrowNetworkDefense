import pandas as pd
import json
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
import argparse
import glob
import os

def load_run_metrics(artifact_dir):
    """
    Recursively find all step_metrics.csv files in the artifact directory,
    load them, and extract population stats.
    """
    all_files = glob.glob(str(Path(artifact_dir) / "**" / "step_metrics.csv"), recursive=True)
    
    if not all_files:
        print(f"No step_metrics.csv files found in {artifact_dir}")
        return pd.DataFrame()

    dfs = []
    print(f"Found {len(all_files)} run metrics files.")

    for filename in all_files:
        path = Path(filename)
        # Try to infer generation and candidate from path structure: .../genXXX/candidateID/step_metrics.csv
        try:
            parts = path.parts
            # Assuming structure ends with genXXX/candidateID/step_metrics.csv
            gen_str = parts[-3]
            candidate_id = parts[-2]
            
            if gen_str.startswith("gen"):
                generation = int(gen_str.replace("gen", ""))
            else:
                generation = 0 # Fallback
        except Exception:
            generation = 0
            candidate_id = "unknown"

        try:
            df = pd.read_csv(filename)
            if 'population_stats' not in df.columns:
                print(f"Skipping {filename}: 'population_stats' column missing.")
                continue

            # Filter rows where population_stats is not null/empty
            df = df.dropna(subset=['population_stats'])
            
            # Parse JSON
            stats_list = []
            valid_indices = []
            for idx, row in df.iterrows():
                try:
                    stats = json.loads(row['population_stats'])
                    if stats:
                        stats_list.append(stats)
                        valid_indices.append(idx)
                except (json.JSONDecodeError, TypeError):
                    continue
            
            if not stats_list:
                continue

            stats_df = pd.DataFrame(stats_list)
            stats_df['step'] = df.loc[valid_indices, 'step'].values
            stats_df['generation'] = generation
            stats_df['candidate_id'] = candidate_id
            
            dfs.append(stats_df)
            
        except Exception as e:
            print(f"Error processing {filename}: {e}")

    if not dfs:
        return pd.DataFrame()

    return pd.concat(dfs, ignore_index=True)

def plot_drift(df, output_dir):
    """
    Generate line plots for each genome parameter over generations/steps.
    """
    # We want to see trends. 
    # Option 1: Boxplot of parameters per generation (distribution of averages).
    # Option 2: Line plot of averages over generations.
    
    # Let's do line plot of the mean parameter value per generation.
    
    # Identify parameter columns (exclude step, generation, candidate_id)
    param_cols = [c for c in df.columns if c not in ['step', 'generation', 'candidate_id']]
    
    if not param_cols:
        print("No genome parameters found to plot.")
        return

    # Calculate average per generation
    gen_means = df.groupby('generation')[param_cols].mean().reset_index()
    
    # Melting for seaborn
    melted = gen_means.melt(id_vars=['generation'], value_vars=param_cols, 
                            var_name='Parameter', value_name='Average Value')

    # Plot 1: All parameters normalized/together? 
    # Scales might differ, so maybe separate plots or faceted.
    
    # Faceted Plot
    g = sns.FacetGrid(melted, col="Parameter", col_wrap=3, height=3, sharey=False)
    g.map(sns.lineplot, "generation", "Average Value", marker="o")
    g.set_titles("{col_name}")
    g.tight_layout()
    
    output_path = Path(output_dir) / "genome_drift_trends.png"
    g.savefig(output_path)
    print(f"Saved drift plot to {output_path}")

    # Plot 2: Detailed view of reproduction_threshold (key parameter)
    if 'avg_reproduction_threshold' in param_cols:
        plt.figure(figsize=(10, 6))
        sns.boxplot(data=df, x='generation', y='avg_reproduction_threshold')
        plt.title('Drift of Reproduction Threshold over Generations')
        plt.savefig(Path(output_dir) / "reproduction_threshold_drift.png")
        print(f"Saved reproduction threshold plot to {Path(output_dir) / 'reproduction_threshold_drift.png'}")

def main():
    parser = argparse.ArgumentParser(description="Visualize evolutionary genome drift.")
    parser.add_argument("--artifact-dir", type=str, default="target/adversarial_runs", 
                        help="Directory containing run artifacts.")
    parser.add_argument("--output-dir", type=str, default="docs/images",
                        help="Directory to save plots.")
    
    args = parser.parse_args()
    
    print(f"Loading metrics from {args.artifact_dir}...")
    df = load_run_metrics(args.artifact_dir)
    
    if df.empty:
        print("No valid genome data found. Run a simulation with multiple generations first.")
        return
        
    print(f"Loaded {len(df)} data points across {df['generation'].nunique()} generations.")
    print("Genome parameters found:", [c for c in df.columns if c not in ['step', 'generation', 'candidate_id']])

    Path(args.output_dir).mkdir(parents=True, exist_ok=True)
    plot_drift(df, args.output_dir)

if __name__ == "__main__":
    main()
