FROM nginx
ARG PORT
COPY nginx.conf.template /
RUN sed -e "s/@PORT@/${PORT}/g" nginx.conf.template >/etc/nginx/conf.d/default.conf
COPY nginx.conf /etc/nginx
