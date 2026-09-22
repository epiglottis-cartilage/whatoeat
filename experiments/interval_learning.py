"""Synthetic feasibility experiment, not a production recommendation service.

Run: python3 experiments/interval_learning.py
The simulated user deliberately follows the proposed model. Results check the
update and sampler together; they are not evidence about real eating behaviour.
"""

import json
import math
import random
import statistics


TARGET = 0.7
SCALE = 0.5
INITIAL_DAYS = 7.0
EXPLORATION = 0.1
MIN_THETA = math.log1p(0.25)
MAX_THETA = math.log1p(180.0)


def probability(days, theta):
    z = math.log(TARGET / (1 - TARGET)) + (math.log1p(days) - theta) / SCALE
    return 1 / (1 + math.exp(-z))


def update(theta, days, accepted, count):
    predicted = probability(days, theta)
    rate = 0.35 / (1 + count / 8) ** 0.65
    return min(MAX_THETA, max(MIN_THETA, theta + rate * (predicted - accepted)))


def simulate(seed, meals):
    rng = random.Random(seed)
    actual = [2.0, 7.0, 20.0]
    theta = [math.log1p(INITIAL_DAYS)] * len(actual)
    # A known meal at day zero supplies each option's initial reference time.
    last_eaten = [0.0] * len(actual)
    counts = [0] * len(actual)
    for meal in range(1, meals + 1):
        now = meal / 2  # two decision sessions per day
        remaining = list(range(len(actual)))
        while remaining:
            weights = [probability(now - last_eaten[i], theta[i]) ** 2 for i in remaining]
            total = sum(weights)
            choices = [(1 - EXPLORATION) * w / total + EXPLORATION / len(remaining)
                       for w in weights]
            chosen = rng.choices(remaining, weights=choices, k=1)[0]
            gap = now - last_eaten[chosen]
            accepted = int(rng.random() < probability(gap, math.log1p(actual[chosen])))
            theta[chosen] = update(theta[chosen], gap, accepted, counts[chosen])
            counts[chosen] += 1
            if accepted:
                last_eaten[chosen] = now
                break
            remaining.remove(chosen)
    return [math.expm1(t) for t in theta], counts


def checks():
    theta = math.log1p(INITIAL_DAYS)
    assert abs(probability(INITIAL_DAYS, theta) - TARGET) < 1e-12
    assert probability(1, theta) < probability(7, theta) < probability(21, theta)
    assert update(theta, 7, 1, 0) < theta < update(theta, 7, 0, 0)
    assert update(theta, 1, 1, 0) < update(theta, 21, 1, 0)
    assert update(theta, 1, 0, 0) < update(theta, 21, 0, 0)
    assert abs(update(theta, 7, 0, 100) - theta) < abs(update(theta, 7, 0, 0) - theta)
    assert update(MAX_THETA, 180, 0, 0) == MAX_THETA
    assert update(MIN_THETA, 0.25, 1, 0) == MIN_THETA


if __name__ == "__main__":
    checks()
    report = {"synthetic_true_intervals_days": [2, 7, 20], "seeds": 30, "runs": []}
    for meals in (120, 1000, 10000):
        results = [simulate(seed, meals) for seed in range(30)]
        estimates = list(zip(*(r[0] for r in results)))
        feedback = list(zip(*(r[1] for r in results)))
        report["runs"].append({
            "meal_sessions": meals,
            "median_estimated_days": [round(statistics.median(x), 2) for x in estimates],
            "p10_p90_estimated_days": [
                [round(sorted(x)[2], 2), round(sorted(x)[26], 2)] for x in estimates
            ],
            "median_feedback_per_option": [statistics.median(x) for x in feedback],
        })
    print(json.dumps(report, ensure_ascii=False, indent=2))
