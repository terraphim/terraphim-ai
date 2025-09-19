current_dir=${PWD}
echo "Current folder ${current_dir}"
# this is to install and build front end inside bionic
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.38.0/install.sh | bash
bash -c "source $HOME/.nvm/nvm.sh && nvm install 20"
bash -c "source $HOME/.nvm/nvm.sh && npm install -g yarn"
bash -c "source $HOME/.nvm/nvm.sh && cd ${current_dir} && yarn && yarn build"
echo "${current_dir}/dist ${current_dir}/../terraphim_server/"
cp -Rv ${current_dir}/dist ${current_dir}/../terraphim_server/
