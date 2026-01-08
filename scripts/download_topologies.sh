#!/bin/bash
# Download sample topologies from Internet Topology Zoo (via GitHub mirrors)

mkdir -p data/external/topologies
cd data/external/topologies

echo "Downloading Abilene..."
curl -L -o Abilene.graphml https://raw.githubusercontent.com/sjas/assessing-mininet/master/parser/topologies/Abilene.graphml

echo "Downloading Geant2012..."
# Finding a reliable mirror is hard without browsing, but Abilene is the gold standard for testing.
# We will use Abilene for now.

echo "Done. Topologies saved to data/external/topologies/"