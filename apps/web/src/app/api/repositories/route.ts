import { NextResponse } from "next/server";

export function GET() {
  return NextResponse.json({
    repositories: [],
    outcome: "empty",
    limitations: ["Repository discovery is not part of the Day 1 local-source slice."],
  });
}