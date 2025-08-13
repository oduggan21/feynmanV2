import { type AxiosRequestConfig } from "axios";

export const axios: AxiosRequestConfig = {
  baseURL: import.meta.env.VITE_API_URL || "http://localhost:3000",
  headers: {
    "Content-Type": "application/json",
    // TEMPORYARY
    "x-user-id": "user_placeholder",
  },
};
