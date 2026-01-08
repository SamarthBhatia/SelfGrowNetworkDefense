#!/bin/bash
# Download sample topologies from Internet Topology Zoo (via GitHub mirrors)

mkdir -p data/external/topologies
cd data/external/topologies

echo "Downloading Abilene..."
curl -L -o Abilene.graphml https://raw.githubusercontent.com/sjas/assessing-mininet/master/parser/topologies/Abilene.graphml

echo "Downloading Geant2012..."
curl -L -o Geant2012.graphml https://raw.githubusercontent.com/PacktPublishing/Network-Science-with-Python-and-NetworkX-Quick-Start-Guide/master/data/UAITZ/Geant2012.graphml

# echo "Downloading Sprint..."
# curl -L -o Sprint.graphml https://raw.githubusercontent.com/topology-zoo/dataset/master/graphml/Sprint.graphml

echo "Done. Topologies saved to data/external/topologies/"