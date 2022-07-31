#!/bin/bash

rm -rf ~/.vim/pack/plugins/start/bus.vim
mkdir -p ~/.vim/pack/plugins/start/bus.vim

PLUGIN_DIR=ftdetect
mkdir -p ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR
install -m 775 ./util/bus/vim/$PLUGIN_DIR/* ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR/

PLUGIN_DIR=ftplugin
mkdir -p ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR
install -m 775 ./util/bus/vim/$PLUGIN_DIR/* ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR/

PLUGIN_DIR=syntax
mkdir -p ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR
install -m 775 ./util/bus/vim/$PLUGIN_DIR/* ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR/

PLUGIN_DIR=indent
mkdir -p ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR
install -m 775 ./util/bus/vim/$PLUGIN_DIR/* ~/.vim/pack/plugins/start/bus.vim/$PLUGIN_DIR/
