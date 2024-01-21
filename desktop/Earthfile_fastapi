VERSION 0.7
PROJECT applied-knowledge-systems/terraphim-cloud-fastapi
FROM ubuntu:18.04
IMPORT ./frontend-svelte AS frontend

ci-pipeline:
  PIPELINE --push
  TRIGGER push main
  TRIGGER pr main
  BUILD +release

build-fastapi:
  FROM ghcr.io/applied-knowledge-systems/redismod:bionic
  RUN apt remove python-pip
  RUN apt install -y --no-install-recommends python3-pip
  RUN apt-get install -y --no-install-recommends  python3-venv
  RUN apt-get install -y --no-install-recommends python3.8-minimal python3.8-dev python3.8-venv
  RUN update-alternatives --install /usr/bin/python python /usr/bin/python3.8 10
  RUN python3.8 -m pip install pip
  WORKDIR /fastapiapp
  COPY --dir defaults .
  COPY main.py models.py requirements.txt .
  COPY --dir service .
  COPY .env .
  RUN python3.8 -m venv ./venv_terraphim_cloud
  RUN /fastapiapp/venv_terraphim_cloud/bin/python3.8 -m pip install -U pip
  RUN /fastapiapp/venv_terraphim_cloud/bin/python3.8 -m pip install -r /fastapiapp/requirements.txt
  SAVE ARTIFACT /fastapiapp /fastapiapp

release:
  FROM +build-fastapi
  WORKDIR /fastapiapp
  # COPY +build-fastapi/fastapiapp .
  COPY frontend+build/dist/assets ./assets
  COPY frontend+build/dist/index.html ./assets/index.html
  COPY service/fastapi.service /etc/systemd/system/fastapi.service
  SAVE ARTIFACT /fastapiapp /fastapiapp
  SAVE IMAGE --push ghcr.io/applied-knowledge-systems/terraphim-fastapiapp:bionic

save-fe-local:
  FROM +build-fastapi
  COPY frontend+build/dist/assets ./assets
  COPY frontend+build/dist/index.html ./assets/index.html
  SAVE ARTIFACT ./assets AS LOCAL ./assets
