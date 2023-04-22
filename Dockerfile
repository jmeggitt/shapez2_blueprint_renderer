# Setup container
FROM ubuntu:22.04
USER root
WORKDIR /home/app

# Install nodejs
# Reference: https://askubuntu.com/a/1113339
RUN apt-get update
RUN apt-get -y install curl gnupg
RUN curl -sL https://deb.nodesource.com/setup_18.x  | bash -
RUN apt-get -y install nodejs

# Allow graphics
RUN apt-get install -y build-essential libxi-dev libglu1-mesa-dev libglew-dev xvfb

# Copy models into container
COPY models/* models/

# Copy and install dependencies
COPY package.json .
RUN npm install

# Copy source files
COPY *.js ./

CMD xvfb-run -s "-ac -screen 0 1280x1024x24" node blueprint_render.js
