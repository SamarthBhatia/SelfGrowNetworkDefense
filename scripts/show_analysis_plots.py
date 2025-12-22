
import json
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
import argparse

def load_harness_data(analysis_name, strategy_name):
    """Load fitness data from the harness state for a given strategy."""
    base_path = Path(f"target/{analysis_name}/{strategy_name}")
    harness_path = base_path / "harness_state.json"
    
    if not harness_path.exists():
        print(f"Harness state not found for {strategy_name} at {harness_path}")
        return pd.DataFrame(columns=['generation', 'fitness_score', 'strategy'])

    with open(harness_path, 'r') as f:
        harness_state = json.load(f)
    
    outcomes = harness_state.get('archive', [])
    
    data = []
    for outcome in outcomes:
        candidate = outcome.get('candidate', {})
        if 'generation' in candidate and 'fitness_score' in outcome:
            data.append({
                'generation': candidate.get('generation'),
                'fitness_score': outcome.get('fitness_score'),
                'strategy': strategy_name
            })
        else:
            print(f"Skipping malformed outcome in {strategy_name}: {outcome}")
            
    if not data:
        print(f"No valid data found in {strategy_name}")
        return pd.DataFrame(columns=['generation', 'fitness_score', 'strategy'])
        
    return pd.DataFrame(data)

def main():
    """Load data, generate and show plots."""
    parser = argparse.ArgumentParser(description='Generate plots for mutation strategy analysis.')
    parser.add_argument('--analysis-name', type=str, default='targeted_mutation_analysis',
                        help='The name of the analysis to generate plots for.')
    args = parser.parse_args()

    # Set plot style
    sns.set_theme(style="whitegrid")

    # Load data for all strategies
    df_random = load_harness_data(args.analysis_name, 'random')
    df_targeted = load_harness_data(args.analysis_name, 'targeted')
    df_hybrid = load_harness_data(args.analysis_name, 'hybrid')

    print("--- Random Strategy Data Head ---")
    print(df_random.head())
    print("\n--- Targeted Strategy Data Head ---")
    print(df_targeted.head())
    print("\n--- Hybrid Strategy Data Head ---")
    print(df_hybrid.head())
    print("\n")

    # Combine into a single DataFrame
    df_combined = pd.concat([df_random, df_targeted, df_hybrid])

    if df_combined.empty:
        print("No data loaded. Cannot generate plots.")
        return

    df_combined = df_combined.dropna(subset=['generation', 'fitness_score'])
    if df_combined.empty:
        print("Data became empty after dropping rows with missing values.")
        return
        
    df_combined['generation'] = df_combined['generation'].astype(int)

    # --- Data processing and aggregation ---
    avg_fitness = df_combined.groupby(['generation', 'strategy'])['fitness_score'].mean().reset_index()
    max_fitness = df_combined.groupby(['generation', 'strategy'])['fitness_score'].max().reset_index()

    print("--- Average Fitness per Generation ---")
    print(avg_fitness.to_string())
    print("\n--- Maximum Fitness per Generation ---")
    print(max_fitness.to_string())
    print("\n")

    # --- Plotting ---
    plt.style.use('seaborn-v0_8-whitegrid')
    fig, axes = plt.subplots(3, 1, figsize=(12, 18), sharex=True)
    fig.suptitle('Comparison of Mutation Strategies', fontsize=16)

    # 1. Average Fitness per Generation
    sns.lineplot(data=avg_fitness, x='generation', y='fitness_score', hue='strategy', ax=axes[0], marker='o')
    axes[0].set_title('Average Fitness per Generation')
    axes[0].set_ylabel('Average Fitness Score')
    axes[0].set_xlabel('')

    # 2. Maximum Fitness per Generation
    sns.lineplot(data=max_fitness, x='generation', y='fitness_score', hue='strategy', ax=axes[1], marker='o')
    axes[1].set_title('Maximum Fitness per Generation')
    axes[1].set_ylabel('Maximum Fitness Score')
    axes[1].set_xlabel('')

    # 3. Distribution of Fitness Scores per Generation
    sns.boxplot(data=df_combined, x='generation', y='fitness_score', hue='strategy', ax=axes[2])
    axes[2].set_title('Distribution of Fitness Scores per Generation')
    axes[2].set_ylabel('Fitness Score')
    axes[2].set_xlabel('Generation')

    plt.tight_layout(rect=[0, 0, 1, 0.96])
    
    # Create a directory for plots
    plot_dir = Path("docs/images")
    plot_dir.mkdir(parents=True, exist_ok=True)

    # Save the plot
    plot_filename = f"{args.analysis_name}_comparison.png"
    fig.savefig(plot_dir / plot_filename)
    print(f"Plots saved to {plot_dir / plot_filename}")

if __name__ == "__main__":
    main()
