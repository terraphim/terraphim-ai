!/usr/bin/env bash
# this is to install and build front end inside bionic
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
bash -c "source $HOME/.nvm/nvm.sh && nvm install 16.15.1"
bash -c "source $HOME/.nvm/nvm.sh && npm install -g yarn"
bash -c "source $HOME/.nvm/nvm.sh && cd /code/desktop && yarn && yarn build"
cp -Rv /code/desktop/dist /code/terraphim_server/