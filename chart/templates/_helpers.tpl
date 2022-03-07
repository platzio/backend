{{/*
Expand the name of the chart.
*/}}
{{- define "chart.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "chart.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "chart.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "chart.labels" -}}
helm.sh/chart: {{ include "chart.chart" . }}
{{ include "chart.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "chart.selectorLabels" -}}
app.kubernetes.io/name: {{ include "chart.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "chart.backendImage" -}}
{{- if .Values.images.backend.image }}
{{- .Values.images.backend.image }}
{{- else }}
{{- .Values.images.backend.repository }}:{{ .Values.images.backend.tag | default .Chart.AppVersion }}
{{- end }}
{{- end }}

{{- define "chart.helmImage" -}}
{{- if .Values.images.helm.image }}
{{- .Values.images.helm.image }}
{{- else }}
{{- .Values.images.helm.repository }}:{{ .Values.images.helm.tag | default .Chart.AppVersion }}
{{- end }}
{{- end }}

{{- define "chart.databaseUrl" -}}
{{- $dbUsername := .Values.postgresql.postgresqlUsername -}}
{{- $dbPassword := .Values.postgresql.postgresqlPassword -}}
{{- $dbHostname := printf "%s-%s.%s.svc" .Release.Name "postgresql" .Release.Namespace -}}
{{- $dbPort     := "5432" -}}
{{- $dbDatabase := .Values.postgresql.postgresqlDatabase -}}
{{- printf "postgres://%s:%s@%s:%s/%s" $dbUsername $dbPassword $dbHostname $dbPort $dbDatabase }}
{{- end }}

{{- define "platz.ownUrlSchema" -}}
    {{- if .enabled }}
        {{- if empty .tls }}
        {{- printf "http" }}
        {{- else if empty (first .tls).hosts }}
        {{- printf "http" }}
        {{- else }}
        {{- printf "https" }}
        {{- end }}
    {{- end }}
{{- end }}

{{- define "platz.ownUrl" -}}
    {{- with .Values.ingress }}
        {{- if .enabled }}
            {{- $scheme := include "platz.ownUrlSchema" . -}}
            {{- $host := (first .hosts).host -}}
            {{- printf "%s://%s" $scheme $host }}
        {{- end }}
    {{- end }}
{{- end }}
