export const ROLES = {
	ADMIN: "admin",
	ORGANIZER: "organizer",
	JUDGE: "judge",
	SPONSOR: "sponsor",
	PARTICIPANT: "participant",
	APPLICANT: "applicant",
} as const;

export type Role = (typeof ROLES)[keyof typeof ROLES];

// Routes without the /h/[hackathonId] prefix
export const ROUTE_PERMISSIONS: Record<string, Role[]> = {
	"/dashboard": ["admin", "organizer", "judge", "sponsor", "participant"],
	"/configuration": ["admin"],
	"/participants": ["admin", "organizer"],
	"/schedule": ["admin", "organizer", "judge", "sponsor", "participant"],
	"/messages": ["admin", "organizer", "judge", "sponsor", "participant"],
	"/judging": ["admin", "organizer", "judge", "sponsor"],
	"/results": ["admin", "organizer", "judge", "sponsor"],
	"/submission": ["admin", "participant"],
	"/check-in": ["admin", "organizer", "participant"],
	"/profile": ["admin", "organizer", "judge", "sponsor", "participant"],
	"/application": ["admin", "applicant"],
	"/team": ["admin", "participant", "applicant"],
};

export const HOME_ROUTES: Record<Role, string> = {
	admin: "/dashboard",
	organizer: "/dashboard",
	judge: "/dashboard",
	sponsor: "/dashboard",
	participant: "/dashboard",
	applicant: "/application",
};

// Remove /h/[hackathonId] prefix
export function extractRoutePath(fullPath: string): string {
	const match = fullPath.match(/^\/h\/[^/]+(\/.*)/);
	if (match) {
		return match[1];
	}

	// Not a hackathon route, return as-is
	return fullPath;
}

export function canAccessRoute(userRole: string, route: string): boolean {
	const routePath = extractRoutePath(route);
	const allowedRoles = ROUTE_PERMISSIONS[routePath];
	if (!allowedRoles) return true;

	return allowedRoles.includes(userRole as Role);
}

// Get the home route for a role within a specific hackathon
export function getHomeRoute(userRole: string, hackathonId: string): string {
	const baseRoute = HOME_ROUTES[userRole as Role] || "/dashboard";
	return `/h/${hackathonId}${baseRoute}`;
}
