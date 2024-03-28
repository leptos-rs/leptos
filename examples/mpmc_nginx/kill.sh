lsof -ti :3000 | xargs kill && \
lsof -ti :3001 | xargs kill &&  \
lsof -ti :3002 | xargs kill && \
lsof -ti :3003 | xargs kill && \
lsof -ti :80 | xargs kill